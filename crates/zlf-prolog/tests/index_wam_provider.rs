use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

use zlf_core::EntityRef;
use zlf_index::{
    bge_m3_dense_v1, content_fingerprint, BM25Index, ExactVectorStore, GenerationId,
    IndexDocumentId, TemporalEntry, TemporalIndex, VectorKey, VectorRecord,
    VECTOR_RECORD_SCHEMA_VERSION,
};
use zlf_prolog::wam::{IndexFactProvider, WamRuntime};
use zlf_prolog::{PrologParser, Term};

#[test]
fn bm25_provider_uses_jieba_chinese_tokenization_in_prolog_query() {
    let dir = tempfile::tempdir().unwrap();
    let bm25 = BM25Index::open(dir.path().join("bm25")).unwrap();
    bm25.index_text("doc1", "软件工程师").unwrap();
    bm25.index_text("doc2", "苹果公司").unwrap();
    let provider = IndexFactProvider::new().with_bm25(&bm25);
    let runtime = WamRuntime::new(12);

    let solutions = runtime
        .query_all_with_provider(&term("bm25(\"软件\", Node, Score)"), &provider)
        .unwrap();

    assert_eq!(solutions[0].get("Node"), Some(&atom("doc1")));
    assert!(solutions[0].contains_key("Score"));
}

#[test]
fn vector_provider_exposes_similarity_to_prolog_query() {
    let dir = tempfile::tempdir().unwrap();
    let vector = ExactVectorStore::open(dir.path().join("vector")).unwrap();
    let mut profile = bge_m3_dense_v1();
    profile.dimension = 2;
    vector
        .put(&vector_record("alice", vec![1.0, 0.0], &profile), &profile)
        .unwrap();
    let norm = (0.9_f32.powi(2) + 0.1_f32.powi(2)).sqrt();
    vector
        .put(
            &vector_record("bob", vec![0.9 / norm, 0.1 / norm], &profile),
            &profile,
        )
        .unwrap();
    let generation = GenerationId("g1".into());
    let provider = IndexFactProvider::new().with_exact_vector(&vector, &profile, &generation);
    let runtime = WamRuntime::new(12);

    let solutions = runtime
        .query_all_with_provider(&term("vector_similar(alice, Node, Score)"), &provider)
        .unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("Node"), Some(&atom("bob")));
}

#[test]
fn temporal_provider_exposes_date_queries_to_prolog() {
    let dir = tempfile::tempdir().unwrap();
    let temporal = TemporalIndex::open(dir.path().join("temporal")).unwrap();
    temporal
        .add_entry(temporal_entry("alice", "2026-01-01T00:00:00Z"))
        .unwrap();
    temporal
        .add_entry(temporal_entry("bob", "2026-02-01T00:00:00Z"))
        .unwrap();
    let provider = IndexFactProvider::new().with_temporal(&temporal);
    let runtime = WamRuntime::new(12);

    let on_date = runtime
        .query_all_with_provider(&term("temporal_on(\"2026-01-01\", Node)"), &provider)
        .unwrap();
    let in_range = runtime
        .query_all_with_provider(
            &term("temporal_between(\"2026-01-01\", \"2026-12-31\", Node)"),
            &provider,
        )
        .unwrap();

    assert_eq!(on_date[0].get("Node"), Some(&atom("alice")));
    assert_eq!(in_range.len(), 2);
}

fn vector_record(
    node_id: &str,
    values: Vec<f32>,
    profile: &zlf_index::EmbeddingModelProfile,
) -> VectorRecord {
    VectorRecord {
        schema_version: VECTOR_RECORD_SCHEMA_VERSION,
        key: VectorKey {
            generation: GenerationId("g1".into()),
            model_profile: profile.id.clone(),
            model_version: profile.version,
            document_id: IndexDocumentId::new(EntityRef::Node(node_id.into()), "body", "0"),
        },
        source_version: 1,
        content_fingerprint: content_fingerprint(node_id),
        model_revision: profile.model_revision.clone(),
        metric: profile.metric,
        normalized: profile.normalize,
        values,
        metadata: BTreeMap::new(),
    }
}

fn temporal_entry(node_id: &str, timestamp: &str) -> TemporalEntry {
    TemporalEntry {
        node_id: node_id.to_string(),
        valid_from: DateTime::parse_from_rfc3339(timestamp)
            .unwrap()
            .with_timezone(&Utc),
        valid_to: None,
    }
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}
