use std::path::Path;

use chrono::{DateTime, Utc};
use zlf_core::Result;
use zlf_index::{
    bge_m3_dense_v1, EmbeddingModelProfile, ExactVectorStore, GenerationId, GenerationMetadata,
    GenerationState, GENERATION_SCHEMA_VERSION,
};
use zlf_storage::Storage;

use crate::{
    BatchEmbeddingProvider, CoordinatorConfig, DurableEmbeddingWorker, EmbeddingModelProfileStore,
    GenerationManager, IndexCoordinator, VectorEmbeddingTarget, ZlfDatabase,
};

const TARGET: &str = "vector";
const BACKEND_SCHEMA: &str = "rocksdb-exact-vector-v1";

pub(crate) struct VectorRuntimeParts {
    pub store: ExactVectorStore,
    pub generation: GenerationId,
    pub model: EmbeddingModelProfile,
}

pub(crate) fn open_active_generation(storage: &Storage, root: &Path) -> Result<VectorRuntimeParts> {
    let profile = bge_m3_dense_v1();
    EmbeddingModelProfileStore::new(storage).put(&profile)?;
    let manager = GenerationManager::new(storage);
    if let Some(active) = manager.active(TARGET)? {
        let store = ExactVectorStore::open(generation_path(root, &active.id))?;
        return Ok(VectorRuntimeParts {
            store,
            generation: active.id,
            model: profile,
        });
    }
    bootstrap_generation(storage, root, profile)
}

fn bootstrap_generation(
    storage: &Storage,
    root: &Path,
    profile: EmbeddingModelProfile,
) -> Result<VectorRuntimeParts> {
    let manager = GenerationManager::new(storage);
    let id = GenerationId("bootstrap-v1".into());
    let metadata = GenerationMetadata {
        schema_version: GENERATION_SCHEMA_VERSION,
        id: id.clone(),
        target: TARGET.into(),
        profile_name: profile.id.clone(),
        profile_version: profile.version,
        backend_schema: BACKEND_SCHEMA.into(),
        source_snapshot_sequence: storage.latest_mutation_sequence()?,
        state: GenerationState::Draft,
        build_checkpoint: 0,
        document_count: 0,
        checksum: None,
        failure: None,
        created_at: Utc::now(),
        validated_at: None,
    };
    manager.create(&metadata)?;
    manager.start_build(TARGET, &id)?;
    let store = ExactVectorStore::open(generation_path(root, &id))?;
    manager.begin_validation(TARGET, &id)?;
    manager.validation_passed(TARGET, &id, 0, "empty-bootstrap")?;
    manager.activate(TARGET, &id)?;
    Ok(VectorRuntimeParts {
        store,
        generation: id,
        model: profile,
    })
}

impl ZlfDatabase {
    pub async fn process_embedding_batch<P: BatchEmbeddingProvider>(
        &self,
        provider: &P,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let target = VectorEmbeddingTarget::new(
            self.vector.as_ref(),
            self.vector_generation.clone(),
            self.vector_model.clone(),
        )?;
        DurableEmbeddingWorker::new(
            &self.storage,
            self.vector.as_ref().clone(),
            provider,
            self.vector_model.clone(),
            target.manifest_scope(),
        )?
        .run_batch(now)
        .await
    }

    pub async fn embed_query_text<P: BatchEmbeddingProvider>(
        &self,
        provider: &P,
        text: &str,
    ) -> Result<Vec<f32>> {
        let target = VectorEmbeddingTarget::new(
            self.vector.as_ref(),
            self.vector_generation.clone(),
            self.vector_model.clone(),
        )?;
        DurableEmbeddingWorker::new(
            &self.storage,
            self.vector.as_ref().clone(),
            provider,
            self.vector_model.clone(),
            target.manifest_scope(),
        )?
        .embed_query(text)
        .await
    }

    pub(crate) fn catch_up_vector(&self) -> Result<()> {
        let coordinator = IndexCoordinator::new(&self.storage, CoordinatorConfig::default());
        coordinator.register_target(TARGET)?;
        let target = VectorEmbeddingTarget::new(
            self.vector.as_ref(),
            self.vector_generation.clone(),
            self.vector_model.clone(),
        )?;
        loop {
            let enqueued = coordinator.enqueue_available(TARGET)?;
            while coordinator.process_next(TARGET, &target)? {}
            if enqueued == 0 {
                break;
            }
        }
        Ok(())
    }

    pub(crate) fn catch_up_indexes(&self) -> Result<()> {
        self.catch_up_bm25()?;
        self.catch_up_vector()?;
        self.catch_up_temporal()
    }
}

fn generation_path(root: &Path, id: &GenerationId) -> std::path::PathBuf {
    root.join("generations").join(&id.0)
}
