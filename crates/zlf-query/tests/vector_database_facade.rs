use std::collections::{BTreeMap, HashMap};

use chrono::Utc;
use zlf_core::{Node, Value};
use zlf_index::{
    ChunkingProfile, EmbeddingModelProfile, EntityMatcher, FieldIndexOptions, IndexProfileArtifact,
    VectorFieldOptions, INDEX_PROFILE_SCHEMA_VERSION,
};
use zlf_query::{BatchEmbeddingProvider, EmbeddingProviderFailure, ZlfDatabase};

#[tokio::test]
async fn database_facade_processes_profile_jobs_and_serves_exact_wam_similarity() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
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
