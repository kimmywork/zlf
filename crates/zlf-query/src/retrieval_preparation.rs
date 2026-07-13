use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use zlf_index::{
    validate_query_vector, GenerationId, RetrievalContractError, RetrievalMode, RetrievalQuery,
    RetrievalRequest,
};

use crate::{BatchEmbeddingProvider, CoordinatorConfig, IndexCoordinator, ZlfDatabase};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PreparedRetrievalHandle(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedIndexSnapshot {
    pub lexical_generation: GenerationId,
    pub lexical_watermark: u64,
    pub vector_generation: GenerationId,
    pub vector_watermark: u64,
    pub temporal_generation: GenerationId,
    pub temporal_watermark: u64,
    pub model_id: String,
    pub model_version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparedRetrieval {
    pub handle: PreparedRetrievalHandle,
    pub request: RetrievalRequest,
    pub query_vector: Option<Vec<f32>>,
    pub snapshot: PreparedIndexSnapshot,
    pub prepared_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum RetrievalPreparationError {
    #[error(transparent)]
    InvalidContract(#[from] RetrievalContractError),
    #[error("prepared retrieval handle not found: {0}")]
    UnknownHandle(String),
    #[error("requested {target} generation {requested} is not active ({active})")]
    GenerationMismatch {
        target: &'static str,
        requested: String,
        active: String,
    },
    #[error("query embedding preparation failed: {0}")]
    Embedding(String),
    #[error("query vector is incompatible with the active model: {0}")]
    InvalidVector(String),
    #[error("minimum watermark {minimum} was not reached for {target}; published {published}")]
    WatermarkTimeout {
        target: String,
        minimum: u64,
        published: u64,
    },
    #[error("retrieval snapshot preparation failed: {0}")]
    Snapshot(String),
}

pub(crate) struct PreparedRetrievalRegistry {
    next_handle: AtomicU64,
    entries: RwLock<BTreeMap<PreparedRetrievalHandle, PreparedRetrieval>>,
}

impl PreparedRetrievalRegistry {
    pub fn new() -> Self {
        Self {
            next_handle: AtomicU64::new(1),
            entries: RwLock::new(BTreeMap::new()),
        }
    }

    fn allocate_handle(&self) -> PreparedRetrievalHandle {
        PreparedRetrievalHandle(format!(
            "prepared-retrieval-{}",
            self.next_handle.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn insert(&self, prepared: PreparedRetrieval) -> Result<(), RetrievalPreparationError> {
        self.entries
            .write()
            .map_err(|error| RetrievalPreparationError::Snapshot(error.to_string()))?
            .insert(prepared.handle.clone(), prepared);
        Ok(())
    }

    fn get(
        &self,
        handle: &PreparedRetrievalHandle,
    ) -> Result<PreparedRetrieval, RetrievalPreparationError> {
        self.entries
            .read()
            .map_err(|error| RetrievalPreparationError::Snapshot(error.to_string()))?
            .get(handle)
            .cloned()
            .ok_or_else(|| RetrievalPreparationError::UnknownHandle(handle.0.clone()))
    }

    fn remove(&self, handle: &PreparedRetrievalHandle) -> Result<bool, RetrievalPreparationError> {
        Ok(self
            .entries
            .write()
            .map_err(|error| RetrievalPreparationError::Snapshot(error.to_string()))?
            .remove(handle)
            .is_some())
    }
}

impl ZlfDatabase {
    pub async fn prepare_retrieval<P: BatchEmbeddingProvider>(
        &self,
        provider: &P,
        request: RetrievalRequest,
    ) -> Result<PreparedRetrievalHandle, RetrievalPreparationError> {
        request.validate()?;
        self.wait_for_retrieval_watermarks(&request)?;
        let snapshot = self.retrieval_snapshot()?;
        validate_requested_generations(&request, &snapshot)?;
        let query_vector = self.prepare_query_vector(provider, &request).await?;
        let handle = self.prepared_retrievals.allocate_handle();
        self.prepared_retrievals.insert(PreparedRetrieval {
            handle: handle.clone(),
            request,
            query_vector,
            snapshot,
            prepared_at: Utc::now(),
        })?;
        Ok(handle)
    }

    pub fn prepared_retrieval(
        &self,
        handle: &PreparedRetrievalHandle,
    ) -> Result<PreparedRetrieval, RetrievalPreparationError> {
        self.prepared_retrievals.get(handle)
    }

    pub fn release_prepared_retrieval(
        &self,
        handle: &PreparedRetrievalHandle,
    ) -> Result<bool, RetrievalPreparationError> {
        self.prepared_retrievals.remove(handle)
    }

    async fn prepare_query_vector<P: BatchEmbeddingProvider>(
        &self,
        provider: &P,
        request: &RetrievalRequest,
    ) -> Result<Option<Vec<f32>>, RetrievalPreparationError> {
        if !matches!(request.mode, RetrievalMode::Vector | RetrievalMode::Hybrid) {
            return Ok(None);
        }
        match &request.query {
            RetrievalQuery::Text { text } => self
                .embed_query_text(provider, text)
                .await
                .map(Some)
                .map_err(|error| RetrievalPreparationError::Embedding(error.to_string())),
            RetrievalQuery::Vector { values, metric } => {
                if *metric != self.vector_model.metric {
                    return Err(RetrievalPreparationError::InvalidVector(
                        "metric does not match the active model".into(),
                    ));
                }
                validate_query_vector(values, &self.vector_model)
                    .map_err(RetrievalPreparationError::InvalidVector)?;
                Ok(Some(values.clone()))
            }
            RetrievalQuery::SourceDocument { .. } | RetrievalQuery::Prepared { .. } => Ok(None),
        }
    }

    fn wait_for_retrieval_watermarks(
        &self,
        request: &RetrievalRequest,
    ) -> Result<(), RetrievalPreparationError> {
        let timeout = std::time::Duration::from_millis(request.wait_timeout_ms);
        for (target, minimum) in &request.minimum_watermarks {
            let result = crate::wait_for_indexes(
                &self.storage,
                std::slice::from_ref(target),
                *minimum,
                timeout,
            )
            .map_err(|error| RetrievalPreparationError::Snapshot(error.to_string()))?;
            if !result.reached {
                let published = IndexCoordinator::new(&self.storage, CoordinatorConfig::default())
                    .progress(target)
                    .map(|progress| progress.published_watermark)
                    .unwrap_or_default();
                return Err(RetrievalPreparationError::WatermarkTimeout {
                    target: target.clone(),
                    minimum: *minimum,
                    published,
                });
            }
        }
        Ok(())
    }

    fn retrieval_snapshot(&self) -> Result<PreparedIndexSnapshot, RetrievalPreparationError> {
        let coordinator = IndexCoordinator::new(&self.storage, CoordinatorConfig::default());
        let watermark = |target| {
            coordinator
                .progress(target)
                .map(|progress| progress.published_watermark)
                .map_err(|error| RetrievalPreparationError::Snapshot(error.to_string()))
        };
        Ok(PreparedIndexSnapshot {
            lexical_generation: self
                .bm25_generation
                .read()
                .map_err(|error| RetrievalPreparationError::Snapshot(error.to_string()))?
                .clone(),
            lexical_watermark: watermark("bm25")?,
            vector_generation: self.vector_generation.clone(),
            vector_watermark: watermark("vector")?,
            temporal_generation: self.temporal_generation.clone(),
            temporal_watermark: watermark("temporal")?,
            model_id: self.vector_model.id.clone(),
            model_version: self.vector_model.version,
        })
    }
}

fn validate_requested_generations(
    request: &RetrievalRequest,
    snapshot: &PreparedIndexSnapshot,
) -> Result<(), RetrievalPreparationError> {
    validate_generation(
        "analyzer",
        request.analyzer_generation.as_ref(),
        &snapshot.lexical_generation,
    )?;
    validate_generation(
        "model",
        request.model_generation.as_ref(),
        &snapshot.vector_generation,
    )
}

fn validate_generation(
    target: &'static str,
    requested: Option<&GenerationId>,
    active: &GenerationId,
) -> Result<(), RetrievalPreparationError> {
    if let Some(requested) = requested {
        if requested != active {
            return Err(RetrievalPreparationError::GenerationMismatch {
                target,
                requested: requested.0.clone(),
                active: active.0.clone(),
            });
        }
    }
    Ok(())
}
