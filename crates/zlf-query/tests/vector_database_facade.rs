use std::collections::{BTreeMap, HashMap};

use chrono::Utc;
use zlf_core::{Node, Value, ZlfError};
use zlf_index::{
    ChunkingProfile, EmbeddingModelProfile, EntityMatcher, FieldIndexOptions, IndexProfileArtifact,
    VectorFieldOptions, INDEX_PROFILE_SCHEMA_VERSION,
};
use zlf_query::{
    BatchEmbeddingProvider, EmbeddingProviderFailure, VectorIndexStrategy, ZlfDatabase,
    ZlfDatabaseOptions,
};

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn database_facade_processes_profile_jobs_and_serves_exact_wam_similarity() {
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
    db.add_node(node("alice", "engineering")).unwrap();
    db.add_node(node("bob", "engineering team")).unwrap();
    db.add_node(node("fruit", "apple")).unwrap();
    assert_eq!(
        db.process_embedding_batch(&FakeProvider, Utc::now())
            .await
            .unwrap(),
        3
    );

    let rows = db
        .query_prolog("? vector_similar(alice, Node, Score).")
        .unwrap();
    assert_eq!(rows[0]["Node"], "bob");
    assert!(rows.iter().all(|row| row["Node"] != "alice"));
    assert_eq!(
        db.embed_query_text(&FakeProvider, "engineering")
            .await
            .unwrap()
            .len(),
        1024
    );
}

#[tokio::test]
async fn embedding_is_disabled_by_default_and_vector_operations_are_explicit_errors() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
    assert!(!db.vector_index_status().enabled);
    assert!(!temp.path().join("vector").exists());
    assert!(matches!(
        db.put_index_profile(&profile()),
        Err(ZlfError::IndexUnavailable { index, .. }) if index == "vector_embedding"
    ));
    assert!(matches!(
        db.process_embedding_batch(&FakeProvider, Utc::now()).await,
        Err(ZlfError::IndexUnavailable { operation, .. })
            if operation == "process_embedding_batch"
    ));
    assert!(matches!(
        db.query_prolog("? vector_similar(alice, Node, Score)."),
        Err(ZlfError::IndexUnavailable { operation, .. })
            if operation == "prolog_vector_query"
    ));
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn hnsw_rebuild_is_async_reopens_and_corruption_falls_back_to_exact() {
    let temp = tempfile::tempdir().unwrap();
    let options = ZlfDatabaseOptions {
        vector_index: VectorIndexStrategy::Hnsw(zlf_index::HnswVectorOptions {
            connections: 8,
            ef_construction: 32,
            max_layer: 16,
            ef_search: 32,
        }),
    };
    let db = ZlfDatabase::open_with_options(temp.path(), options).unwrap();
    db.put_index_profile(&profile()).unwrap();
    db.activate_index_profile("knowledge", 1).unwrap();
    db.add_node(node("alice", "engineering")).unwrap();
    db.add_node(node("bob", "engineering team")).unwrap();
    db.process_embedding_batch(&FakeProvider, Utc::now())
        .await
        .unwrap();
    let started = std::time::Instant::now();
    assert!(db.request_vector_rebuild().unwrap());
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    wait_for_ann(&db);
    assert!(db.vector_index_status().ann_ready);
    assert!(!db.vector_index_status().exact_fallback);
    assert_eq!(
        db.query_prolog("? vector_similar(alice, Node, Score).")
            .unwrap()[0]["Node"],
        "bob"
    );
    drop(db);

    let hnsw_root = temp.path().join("vector/hnsw/bootstrap-v1");
    let publication = std::fs::read_to_string(hnsw_root.join("active")).unwrap();
    std::fs::remove_file(
        hnsw_root
            .join("publications")
            .join(publication.trim())
            .join("vectors.hnsw.data"),
    )
    .unwrap();
    let reopened = ZlfDatabase::open_existing_with_options(temp.path(), options).unwrap();
    assert!(reopened.vector_index_status().exact_fallback);
    assert_eq!(
        reopened
            .query_prolog("? vector_similar(alice, Node, Score).")
            .unwrap()[0]["Node"],
        "bob"
    );
}

fn wait_for_ann(db: &ZlfDatabase) {
    for _ in 0..100 {
        let status = db.vector_index_status();
        if !status.ann_rebuilding {
            assert!(status.ann_ready, "ANN rebuild did not publish");
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("ANN rebuild timed out");
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

fn node(id: &str, body: &str) -> Node {
    Node::with_id(
        id.into(),
        vec!["document".into()],
        HashMap::from([("body".into(), Value::String(body.into()))]),
    )
}

fn profile() -> IndexProfileArtifact {
    let mut profile = IndexProfileArtifact {
        schema_version: INDEX_PROFILE_SCHEMA_VERSION,
        name: "knowledge".into(),
        version: 1,
        source_hash: String::new(),
        matcher: EntityMatcher::NodeLabels {
            labels: vec!["document".into()],
        },
        fields: BTreeMap::from([(
            "body".into(),
            FieldIndexOptions {
                bm25: None,
                vector: Some(VectorFieldOptions {
                    model_profile: "bge_m3_dense_v1".into(),
                    chunking: ChunkingProfile::WholeField { version: 1 },
                }),
                temporal: None,
            },
        )]),
        created_at: Utc::now(),
    };
    profile.refresh_source_hash();
    profile
}
