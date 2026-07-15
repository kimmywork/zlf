use std::collections::{BTreeMap, HashMap};

use chrono::Utc;
use zlf_core::{Node, Value};
use zlf_index::{
    Bm25FieldOptions, ChunkingProfile, EmbeddingModelProfile, EntityMatcher, FieldIndexOptions,
    IndexProfileArtifact, ResultAggregation, RetrievalBudgets, RetrievalMode, RetrievalQuery,
    RetrievalRequest, TemporalFilter, TemporalRole, VectorFieldOptions,
    INDEX_PROFILE_SCHEMA_VERSION,
};
use zlf_query::{
    BatchEmbeddingProvider, EmbeddingProviderFailure, VectorIndexStrategy, ZlfDatabase,
    ZlfDatabaseOptions,
};

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn prepared_hybrid_execution_fuses_pages_and_applies_temporal_and_graph_filters() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open_with_options(
        temp.path(),
        ZlfDatabaseOptions {
            vector_index: VectorIndexStrategy::Exact,
        },
    )
    .unwrap();
    db.put_index_profile(&profile()).unwrap();
    db.activate_index_profile("knowledge", 1).unwrap();
    db.add_node(node("alice", "engineering", "2026-01-01", false))
        .unwrap();
    db.add_node(node("bob", "engineering team", "2026-02-01", true))
        .unwrap();
    db.add_node(node("fruit", "apple", "2025-01-01", true))
        .unwrap();
    assert_eq!(
        db.process_embedding_batch(&FakeProvider, Utc::now())
            .await
            .unwrap(),
        3
    );
    let handle = db
        .prepare_retrieval(&FakeProvider, request())
        .await
        .unwrap();
    let result = db.execute_prepared_retrieval(&handle).unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].document_id.entity.id(), "bob");
    assert!(result.hits[0].lexical.is_some());
    assert!(result.hits[0].vector.is_some());
    assert!(result.metadata.lexical_pages >= 2);
    assert!(result.metadata.vector_pages >= 2);
    assert_eq!(result.metadata.graph_rejected, 1);
    assert_eq!(result.metadata.temporal_rejected, 1);
    assert!(!result.metadata.candidate_budget_exhausted);
    assert!(result.metadata.exact_filtered_top_k);

    let rows = db
        .query_prolog(&format!("? retrieve(\"{}\", {{}}, Entity, Hit).", handle.0))
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["Entity"], "bob");
    assert_eq!(rows[0]["Hit"]["fused_rank"], 1);
    assert_eq!(rows[0]["Hit"]["exact_filtered_top_k"], "true");
    assert_eq!(rows[0]["Hit"]["strategy"], "retrieval_first");
    assert_ne!(rows[0]["Hit"]["lexical_generation"], "none");
    assert_ne!(rows[0]["Hit"]["vector_generation"], "none");
    let bound = db
        .query_prolog(&format!("? retrieve(\"{}\", {{}}, bob, Hit).", handle.0))
        .unwrap();
    assert_eq!(bound.len(), 1);
    assert_eq!(bound[0]["Hit"]["strategy"], "bound_entity");
    let proof = db
        .query_prolog_with_proof(&format!("? retrieve(\"{}\", {{}}, bob, Hit).", handle.0))
        .unwrap();
    assert!(proof[0]
        .proof
        .nodes
        .iter()
        .any(|node| node.clause.kind == zlf_prolog::wam::ProofKind::Index
            && node.clause.id.starts_with("index:retrieve:")));

    for mode in [RetrievalMode::Lexical, RetrievalMode::Vector] {
        let mut single = request();
        single.mode = mode;
        single.top_k = 1;
        single.temporal_filter = None;
        single.graph_filter_goal = None;
        let baseline = db
            .prepare_retrieval(&FakeProvider, single.clone())
            .await
            .unwrap();
        let baseline = db.execute_prepared_retrieval(&baseline).unwrap();
        assert_eq!(baseline.hits[0].document_id.entity.id(), "alice");
        single.exclude_source = Some(baseline.hits[0].document_id.clone());
        let handle = db.prepare_retrieval(&FakeProvider, single).await.unwrap();
        let result = db.execute_prepared_retrieval(&handle).unwrap();
        assert_eq!(result.hits[0].document_id.entity.id(), "bob");
        assert_eq!(
            result.hits[0].lexical.is_some(),
            mode == RetrievalMode::Lexical
        );
        assert_eq!(
            result.hits[0].vector.is_some(),
            mode == RetrievalMode::Vector
        );
    }

    db.query_prolog(":- table retrieve/4.").unwrap();
    db.query_prolog(&format!("? retrieve(\"{}\", {{}}, Entity, Hit).", handle.0))
        .unwrap();
    let hot_hits = db.table_metrics().hot_hits;
    db.query_prolog(&format!("? retrieve(\"{}\", {{}}, Entity, Hit).", handle.0))
        .unwrap();
    assert!(db.table_metrics().hot_hits > hot_hits);
    let error = db
        .query_prolog("? retrieve(Handle, {}, Entity, Hit).")
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("bound prepared handle and options"));
    let invalidations = db.table_metrics().stale_invalidations;
    db.add_node(node("new", "engineering", "2026-03-01", true))
        .unwrap();
    assert!(db.table_metrics().stale_invalidations > invalidations);
}

fn request() -> RetrievalRequest {
    RetrievalRequest {
        query: RetrievalQuery::Text {
            text: "engineering".into(),
        },
        mode: RetrievalMode::Hybrid,
        profiles: vec!["knowledge".into()],
        top_k: 2,
        budgets: RetrievalBudgets {
            candidate_k: 4,
            page_size: 1,
            max_pages: 4,
            max_answers: 2,
        },
        threshold: None,
        fields: vec!["body".into()],
        model_generation: None,
        analyzer_generation: None,
        temporal_filter: Some(TemporalFilter::EventRange {
            start_micros: zlf_index::parse_utc_micros("2026-01-01").unwrap(),
            end_micros: zlf_index::parse_utc_micros("2027-01-01").unwrap(),
        }),
        exclude_source: None,
        graph_filter_goal: Some("label(Entity, allowed)".into()),
        minimum_watermarks: BTreeMap::new(),
        wait_timeout_ms: 1_000,
        aggregation: ResultAggregation::Document,
        explain: true,
    }
}

struct FakeProvider;

#[async_trait::async_trait]
impl BatchEmbeddingProvider for FakeProvider {
    async fn embed_query(
        &self,
        _profile: &EmbeddingModelProfile,
        text: &str,
    ) -> Result<Vec<f32>, EmbeddingProviderFailure> {
        Ok(vector(text))
    }

    async fn embed_documents(
        &self,
        _profile: &EmbeddingModelProfile,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingProviderFailure> {
        Ok(texts.iter().map(|text| vector(text)).collect())
    }
}

fn vector(text: &str) -> Vec<f32> {
    let mut values = vec![0.0; 1024];
    if text.contains("engineering") {
        values[0] = 1.0;
    } else {
        values[1] = 1.0;
    }
    values
}

fn node(id: &str, body: &str, occurred: &str, allowed: bool) -> Node {
    let labels = if allowed {
        vec!["document".into(), "allowed".into()]
    } else {
        vec!["document".into()]
    };
    Node::with_id(
        id.into(),
        labels,
        HashMap::from([
            ("body".into(), Value::String(body.into())),
            ("occurred".into(), Value::String(occurred.into())),
        ]),
    )
}

#[allow(clippy::too_many_lines)]
fn profile() -> IndexProfileArtifact {
    let mut profile = IndexProfileArtifact {
        schema_version: INDEX_PROFILE_SCHEMA_VERSION,
        name: "knowledge".into(),
        version: 1,
        source_hash: String::new(),
        matcher: EntityMatcher::NodeLabels {
            labels: vec!["document".into()],
        },
        fields: BTreeMap::from([
            (
                "body".into(),
                FieldIndexOptions {
                    bm25: Some(Bm25FieldOptions {
                        analyzer_id: "unicode_jieba_v1".into(),
                        language: None,
                        analyzer_version: 1,
                        weight: 1.0,
                        k1: 1.2,
                        b: 0.75,
                    }),
                    vector: Some(VectorFieldOptions {
                        model_profile: "bge_m3_dense_v1".into(),
                        chunking: ChunkingProfile::WholeField { version: 1 },
                    }),
                    temporal: None,
                },
            ),
            (
                "occurred".into(),
                FieldIndexOptions {
                    bm25: None,
                    vector: None,
                    temporal: Some(TemporalRole::Event),
                },
            ),
        ]),
        created_at: Utc::now(),
    };
    profile.refresh_source_hash();
    profile
}
