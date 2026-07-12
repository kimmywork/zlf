use chrono::{DateTime, Duration, Utc};
use zlf_core::{Result, ZlfError};
use zlf_index::{
    EmbeddingJob, EmbeddingModelProfile, ExactVectorStore, VectorKey, VectorRecord,
    VECTOR_RECORD_SCHEMA_VERSION,
};
use zlf_storage::Storage;

use crate::{EmbeddingJobStore, IndexManifestStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingProviderFailure {
    pub class: String,
    pub retryable: bool,
}

#[async_trait::async_trait]
pub trait BatchEmbeddingProvider: Send + Sync {
    async fn embed_query(
        &self,
        profile: &EmbeddingModelProfile,
        text: &str,
    ) -> std::result::Result<Vec<f32>, EmbeddingProviderFailure>;

    async fn embed_documents(
        &self,
        profile: &EmbeddingModelProfile,
        texts: &[String],
    ) -> std::result::Result<Vec<Vec<f32>>, EmbeddingProviderFailure>;
}

#[async_trait::async_trait]
impl<T: zlf_embed::EmbeddingProvider + ?Sized> BatchEmbeddingProvider for T {
    async fn embed_query(
        &self,
        _profile: &EmbeddingModelProfile,
        text: &str,
    ) -> std::result::Result<Vec<f32>, EmbeddingProviderFailure> {
        self.embed(text).await.map_err(embed_failure)
    }

    async fn embed_documents(
        &self,
        _profile: &EmbeddingModelProfile,
        texts: &[String],
    ) -> std::result::Result<Vec<Vec<f32>>, EmbeddingProviderFailure> {
        self.embed_batch(&texts.iter().map(String::as_str).collect::<Vec<_>>())
            .await
            .map_err(embed_failure)
    }
}

pub struct DurableEmbeddingWorker<'a, P> {
    storage: &'a Storage,
    store: ExactVectorStore,
    provider: &'a P,
    profile: EmbeddingModelProfile,
    manifest_scope: String,
    lease: Duration,
    max_attempts: u32,
}

impl<'a, P: BatchEmbeddingProvider> DurableEmbeddingWorker<'a, P> {
    pub fn new(
        storage: &'a Storage,
        store: ExactVectorStore,
        provider: &'a P,
        profile: EmbeddingModelProfile,
        manifest_scope: &str,
    ) -> Result<Self> {
        profile.validate_dense_v1().map_err(ZlfError::Internal)?;
        Ok(Self {
            storage,
            store,
            provider,
            profile,
            manifest_scope: manifest_scope.into(),
            lease: Duration::seconds(30),
            max_attempts: 8,
        })
    }

    pub fn with_policy(mut self, lease: Duration, max_attempts: u32) -> Self {
        self.lease = lease;
        self.max_attempts = max_attempts;
        self
    }

    pub async fn run_batch(&self, now: DateTime<Utc>) -> Result<usize> {
        let current = self.current_jobs(now)?;
        if current.is_empty() {
            return Ok(0);
        }
        let transformed = current
            .iter()
            .map(|(_, text)| {
                self.profile
                    .transform_document(text)
                    .map_err(ZlfError::Internal)
            })
            .collect::<Result<Vec<_>>>()?;
        let vectors = match self
            .provider
            .embed_documents(&self.profile, &transformed)
            .await
        {
            Ok(vectors) if vectors.len() == current.len() => vectors,
            Ok(_) => return self.batch_size_failure(&current, now),
            Err(failure) => return self.fail_batch(&current, failure, now),
        };
        self.publish_batch(&current, vectors, now)
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        let transformed = self
            .profile
            .transform_query(text)
            .map_err(ZlfError::Internal)?;
        let values = self
            .provider
            .embed_query(&self.profile, &transformed)
            .await
            .map_err(provider_error)?;
        normalize_and_validate(values, &self.profile)
    }

    fn current_jobs(&self, now: DateTime<Utc>) -> Result<Vec<(EmbeddingJob, String)>> {
        let jobs = EmbeddingJobStore::new(self.storage).claim_ready(
            now,
            self.profile.batch_limit,
            self.lease,
        )?;
        let mut current = Vec::new();
        for job in jobs {
            if self.is_stale(&job)? {
                EmbeddingJobStore::new(self.storage).stale(&job, now)?;
            } else {
                current.push((job.clone(), self.document_text(&job)?));
            }
        }
        Ok(current)
    }

    fn publish_batch(
        &self,
        jobs: &[(EmbeddingJob, String)],
        vectors: Vec<Vec<f32>>,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let mut published = 0;
        for ((job, _), values) in jobs.iter().zip(vectors) {
            let values = match normalize_and_validate(values, &self.profile) {
                Ok(values) => values,
                Err(_) => {
                    EmbeddingJobStore::new(self.storage).fail(
                        job,
                        "invalid_vector",
                        now,
                        self.max_attempts,
                        false,
                    )?;
                    continue;
                }
            };
            self.publish(job, values)?;
            EmbeddingJobStore::new(self.storage).complete(job, now)?;
            published += 1;
        }
        Ok(published)
    }

    fn batch_size_failure(
        &self,
        jobs: &[(EmbeddingJob, String)],
        now: DateTime<Utc>,
    ) -> Result<usize> {
        self.fail_batch(
            jobs,
            EmbeddingProviderFailure {
                class: "batch_size_mismatch".into(),
                retryable: false,
            },
            now,
        )
    }

    fn publish(&self, job: &EmbeddingJob, values: Vec<f32>) -> Result<()> {
        self.store.put(
            &VectorRecord {
                schema_version: VECTOR_RECORD_SCHEMA_VERSION,
                key: VectorKey {
                    generation: job.generation.clone(),
                    model_profile: job.model_profile.clone(),
                    model_version: job.model_version,
                    document_id: job.document_id.clone(),
                },
                source_version: job.source_version,
                content_fingerprint: job.content_fingerprint.clone(),
                model_revision: self.profile.model_revision.clone(),
                metric: self.profile.metric,
                normalized: self.profile.normalize,
                values,
                metadata: Default::default(),
            },
            &self.profile,
        )
    }

    fn document_text(&self, job: &EmbeddingJob) -> Result<String> {
        IndexManifestStore::new(self.storage, &self.manifest_scope)
            .list_for_entity(&job.document_id.entity)?
            .into_iter()
            .flat_map(|manifest| manifest.documents)
            .find(|document| document.id == job.document_id)
            .map(|document| document.content)
            .ok_or_else(|| ZlfError::Internal("embedding document content not found".into()))
    }

    fn is_stale(&self, job: &EmbeddingJob) -> Result<bool> {
        Ok(self
            .storage
            .get_entity_state(&job.document_id.entity)?
            .is_none_or(|state| state.deleted || state.source_version != job.source_version))
    }

    fn fail_batch(
        &self,
        jobs: &[(EmbeddingJob, String)],
        failure: EmbeddingProviderFailure,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        for (job, _) in jobs {
            EmbeddingJobStore::new(self.storage).fail(
                job,
                &failure.class,
                now + retry_delay(job.attempts),
                self.max_attempts,
                failure.retryable,
            )?;
        }
        Ok(0)
    }
}

fn normalize_and_validate(
    mut values: Vec<f32>,
    profile: &EmbeddingModelProfile,
) -> Result<Vec<f32>> {
    if values.len() != profile.dimension || values.iter().any(|value| !value.is_finite()) {
        return Err(ZlfError::Internal(
            "provider returned an invalid vector".into(),
        ));
    }
    if profile.normalize {
        let norm = values
            .iter()
            .map(|value| f64::from(*value).powi(2))
            .sum::<f64>()
            .sqrt();
        if norm == 0.0 {
            return Err(ZlfError::Internal("provider returned a zero vector".into()));
        }
        values
            .iter_mut()
            .for_each(|value| *value = (f64::from(*value) / norm) as f32);
    }
    Ok(values)
}

fn retry_delay(attempts: u32) -> Duration {
    Duration::seconds(1_i64 << attempts.min(10))
}

fn embed_failure(error: zlf_embed::EmbedError) -> EmbeddingProviderFailure {
    let (class, retryable) = match error {
        zlf_embed::EmbedError::Http(_) => ("http", true),
        zlf_embed::EmbedError::Provider(_) => ("provider", true),
        zlf_embed::EmbedError::Json(_) => ("invalid_json", false),
        zlf_embed::EmbedError::InvalidResponse(_) => ("invalid_response", false),
    };
    EmbeddingProviderFailure {
        class: class.into(),
        retryable,
    }
}

fn provider_error(failure: EmbeddingProviderFailure) -> ZlfError {
    ZlfError::Internal(format!("embedding provider failure: {}", failure.class))
}
