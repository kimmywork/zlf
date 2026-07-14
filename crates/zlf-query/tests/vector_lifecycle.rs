use std::collections::{BTreeMap, HashMap};

use chrono::Utc;
use zlf_core::{Node, Value};
use zlf_index::{
    bge_m3_dense_v1, ChunkingProfile, EmbeddingModelProfile, EntityMatcher, ExactVectorStore,
    FieldIndexOptions, GenerationId, IndexDocumentId, IndexProfileArtifact, VectorFieldOptions,
    VectorKey, INDEX_PROFILE_SCHEMA_VERSION,
};
use zlf_query::{
    BatchEmbeddingProvider, CoordinatorConfig, DurableEmbeddingWorker, EmbeddingJobStore,
    EmbeddingProviderFailure, IndexCoordinator, IndexProfileStore, VectorEmbeddingTarget,
};
use zlf_storage::Storage;

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn lifecycle_enqueues_publishes_replaces_deletes_and_replays_vectors() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    let exact = ExactVectorStore::open(temp.path().join("vectors")).unwrap();
    let model = model();
    storage
        .create_node(Node::with_id(
            "doc".into(),
            vec!["document".into()],
            HashMap::from([("body".into(), Value::String("hello".into()))]),
        ))
        .unwrap();
    let profiles = IndexProfileStore::new(&storage);
    profiles.put(&profile()).unwrap();
    profiles.activate("knowledge", 1).unwrap();
    let coordinator = IndexCoordinator::new(&storage, CoordinatorConfig::default());
    coordinator.register_target("vector").unwrap();
    coordinator.enqueue_available("vector").unwrap();
    let target =
        VectorEmbeddingTarget::new(&exact, GenerationId("g1".into()), model.clone()).unwrap();
    while coordinator.process_next("vector", &target).unwrap() {}
    let queued = EmbeddingJobStore::new(&storage).list().unwrap();
    assert_eq!(queued.len(), 1);
    let initial_key = key(queued[0].document_id.clone());

    let provider = ContentProvider;
    let worker = DurableEmbeddingWorker::new(
        &storage,
        exact.clone(),
        &provider,
        model.clone(),
        target.manifest_scope(),
    )
    .unwrap();
    assert_eq!(worker.run_batch(Utc::now()).await.unwrap(), 1);
    assert_eq!(
        EmbeddingJobStore::new(&storage).state_counts().unwrap()["completed"],
        1
    );
    assert_eq!(
        exact.get(&initial_key).unwrap().unwrap().values,
        vec![1.0, 0.0]
    );

    storage
        .update_node(
            "doc",
            HashMap::from([("body".into(), Value::String("updated".into()))]),
        )
        .unwrap();
    coordinator.enqueue_available("vector").unwrap();
    coordinator.process_next("vector", &target).unwrap();
    assert!(exact.get(&initial_key).unwrap().is_none());
    let updated_job = EmbeddingJobStore::new(&storage)
        .list()
        .unwrap()
        .into_iter()
        .max_by_key(|job| job.source_version)
        .unwrap();
    let updated_key = key(updated_job.document_id);
    assert_eq!(worker.run_batch(Utc::now()).await.unwrap(), 1);
    assert_eq!(
        exact.get(&updated_key).unwrap().unwrap().values,
        vec![0.0, 1.0]
    );

    storage.delete_node("doc").unwrap();
    coordinator.enqueue_available("vector").unwrap();
    coordinator.process_next("vector", &target).unwrap();
    assert!(exact.get(&updated_key).unwrap().is_none());
    coordinator.enqueue_available("vector").unwrap();
    assert!(!coordinator.process_next("vector", &target).unwrap());
}

struct ContentProvider;

#[async_trait::async_trait]
impl BatchEmbeddingProvider for ContentProvider {
    async fn embed_query(
        &self,
        _profile: &EmbeddingModelProfile,
        _text: &str,
    ) -> Result<Vec<f32>, EmbeddingProviderFailure> {
        unreachable!()
    }

    async fn embed_documents(
        &self,
        _profile: &EmbeddingModelProfile,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingProviderFailure> {
        Ok(texts
            .iter()
            .map(|text| {
                if text.contains("updated") {
                    vec![0.0, 1.0]
                } else {
                    vec![1.0, 0.0]
                }
            })
            .collect())
    }
}

fn model() -> EmbeddingModelProfile {
    let mut profile = bge_m3_dense_v1();
    profile.dimension = 2;
    profile
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

fn key(document_id: IndexDocumentId) -> VectorKey {
    VectorKey {
        generation: GenerationId("g1".into()),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        document_id,
    }
}
