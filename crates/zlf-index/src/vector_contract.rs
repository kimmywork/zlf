use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ContentFingerprint, EmbeddingModelProfile, GenerationId, IndexDocumentId, VectorMetric,
};

pub const VECTOR_RECORD_SCHEMA_VERSION: u32 = 1;
pub const EMBEDDING_JOB_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VectorKey {
    pub generation: GenerationId,
    pub model_profile: String,
    pub model_version: u32,
    pub document_id: IndexDocumentId,
}

impl VectorKey {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_part(&mut bytes, self.generation.0.as_bytes());
        push_part(&mut bytes, self.model_profile.as_bytes());
        push_part(&mut bytes, &self.model_version.to_be_bytes());
        push_part(&mut bytes, &self.document_id.canonical_bytes());
        bytes
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorRecord {
    pub schema_version: u32,
    pub key: VectorKey,
    pub source_version: u64,
    pub content_fingerprint: ContentFingerprint,
    pub model_revision: String,
    pub metric: VectorMetric,
    pub normalized: bool,
    pub values: Vec<f32>,
    pub metadata: BTreeMap<String, String>,
}

impl VectorRecord {
    pub fn validate(&self, profile: &EmbeddingModelProfile) -> Result<(), String> {
        profile.validate_dense_v1()?;
        if self.schema_version != VECTOR_RECORD_SCHEMA_VERSION
            || self.key.model_profile != profile.id
            || self.key.model_version != profile.version
            || self.model_revision != profile.model_revision
            || self.metric != profile.metric
            || self.normalized != profile.normalize
            || self.values.len() != profile.dimension
            || self.values.iter().any(|value| !value.is_finite())
        {
            return Err("vector record is incompatible with model profile".into());
        }
        let norm_squared = self
            .values
            .iter()
            .map(|value| f64::from(*value).powi(2))
            .sum::<f64>();
        if profile.metric == VectorMetric::Cosine && norm_squared == 0.0 {
            return Err("cosine vectors must be nonzero".into());
        }
        if profile.normalize && (norm_squared.sqrt() - 1.0).abs() > 1e-4 {
            return Err("vector does not satisfy normalization policy".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorQuery {
    pub generation: GenerationId,
    pub model_profile: String,
    pub model_version: u32,
    pub values: Vec<f32>,
    pub top_k: usize,
    pub threshold: Option<f32>,
    pub include_sources: Vec<IndexDocumentId>,
    pub exclude_sources: Vec<IndexDocumentId>,
    pub include_entities: Vec<zlf_core::EntityRef>,
    pub exclude_entities: Vec<zlf_core::EntityRef>,
    pub metadata: BTreeMap<String, String>,
}

impl VectorQuery {
    pub fn validate(&self, profile: &EmbeddingModelProfile) -> Result<(), String> {
        if self.top_k == 0
            || self.model_profile != profile.id
            || self.model_version != profile.version
            || self.threshold.is_some_and(|value| !value.is_finite())
        {
            return Err("invalid vector query".into());
        }
        validate_query_vector(&self.values, profile)
    }
}

pub fn validate_query_vector(
    values: &[f32],
    profile: &EmbeddingModelProfile,
) -> Result<(), String> {
    if values.len() != profile.dimension || values.iter().any(|value| !value.is_finite()) {
        return Err("query vector is incompatible with model profile".into());
    }
    let norm = values
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    if profile.metric == VectorMetric::Cosine && norm == 0.0 {
        return Err("cosine query vector must be nonzero".into());
    }
    if profile.normalize && (norm - 1.0).abs() > 1e-4 {
        return Err("query vector does not satisfy normalization policy".into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorHit {
    pub key: VectorKey,
    pub score: f32,
    pub source_version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingJobState {
    Pending,
    Leased,
    Retry,
    Dead,
    Completed,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingJob {
    pub schema_version: u32,
    pub generation: GenerationId,
    pub document_id: IndexDocumentId,
    pub source_version: u64,
    pub content_fingerprint: ContentFingerprint,
    pub model_profile: String,
    pub model_version: u32,
    pub expected_dimension: usize,
    pub attempts: u32,
    pub state: EmbeddingJobState,
    pub created_at: DateTime<Utc>,
    pub lease_until: Option<DateTime<Utc>>,
    pub retry_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_error_class: Option<String>,
}

fn push_part(target: &mut Vec<u8>, part: &[u8]) {
    target.extend_from_slice(&(part.len() as u32).to_be_bytes());
    target.extend_from_slice(part);
}

impl EmbeddingJob {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != EMBEDDING_JOB_SCHEMA_VERSION
            || self.generation.0.is_empty()
            || self.model_profile.is_empty()
            || self.model_version == 0
            || self.expected_dimension == 0
            || self
                .last_error_class
                .as_ref()
                .is_some_and(|error| error.len() > 128)
        {
            return Err("invalid embedding job".into());
        }
        Ok(())
    }
}
