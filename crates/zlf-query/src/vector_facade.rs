use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use zlf_core::Result;

use crate::vector_runtime::{disabled_status, VectorIndexStatus};
use crate::{
    BatchEmbeddingProvider, CoordinatorConfig, DurableEmbeddingWorker, EmbeddingJobStore,
    IndexCoordinator, VectorEmbeddingTarget, ZlfDatabase,
};

const TARGET: &str = "vector";

impl ZlfDatabase {
    pub fn vector_index_status(&self) -> VectorIndexStatus {
        self.vector
            .as_ref()
            .map_or_else(disabled_status, |vector| vector.status())
    }

    pub fn request_vector_rebuild(&self) -> Result<bool> {
        self.require_vector("request_rebuild")?.request_rebuild()
    }

    pub fn embedding_job_state_counts(&self) -> Result<BTreeMap<String, usize>> {
        self.require_vector("embedding_job_state_counts")?;
        EmbeddingJobStore::new(&self.storage).state_counts()
    }

    pub async fn process_embedding_batch<P: BatchEmbeddingProvider>(
        &self,
        provider: &P,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let vector = self.require_vector("process_embedding_batch")?;
        vector.mark_stale();
        let target = VectorEmbeddingTarget::new(
            vector.store.as_ref(),
            vector.generation.clone(),
            vector.model.clone(),
        )?;
        let published = DurableEmbeddingWorker::new(
            &self.storage,
            vector.store.as_ref().clone(),
            provider,
            vector.model.clone(),
            target.manifest_scope(),
        )?
        .run_batch(now)
        .await?;
        if published > 0 {
            self.invalidate_retrieval_tables()?;
        }
        Ok(published)
    }

    pub async fn process_embedding_batch_and_request_rebuild<P: BatchEmbeddingProvider>(
        &self,
        provider: &P,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let published = self.process_embedding_batch(provider, now).await?;
        if published > 0 {
            self.request_vector_rebuild()?;
        }
        Ok(published)
    }

    pub async fn embed_query_text<P: BatchEmbeddingProvider>(
        &self,
        provider: &P,
        text: &str,
    ) -> Result<Vec<f32>> {
        let vector = self.require_vector("embed_query_text")?;
        let target = VectorEmbeddingTarget::new(
            vector.store.as_ref(),
            vector.generation.clone(),
            vector.model.clone(),
        )?;
        DurableEmbeddingWorker::new(
            &self.storage,
            vector.store.as_ref().clone(),
            provider,
            vector.model.clone(),
            target.manifest_scope(),
        )?
        .embed_query(text)
        .await
    }

    pub(crate) fn catch_up_vector(&self) -> Result<()> {
        let Some(vector) = self.vector.as_ref() else {
            return Ok(());
        };
        let coordinator = IndexCoordinator::new(&self.storage, CoordinatorConfig::default());
        coordinator.register_target(TARGET)?;
        let target = VectorEmbeddingTarget::new(
            vector.store.as_ref(),
            vector.generation.clone(),
            vector.model.clone(),
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
        self.catch_up_temporal()?;
        self.invalidate_retrieval_tables()
    }
}
