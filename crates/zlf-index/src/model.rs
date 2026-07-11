use serde::{Deserialize, Serialize};

pub const EMBEDDING_MODEL_PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorMetric {
    Cosine,
    DotProduct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingCapabilities {
    pub dense: bool,
    pub sparse: bool,
    pub multi_vector: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingModelProfile {
    pub schema_version: u32,
    pub id: String,
    pub version: u32,
    pub provider: String,
    pub model_id: String,
    pub model_revision: String,
    pub dimension: usize,
    pub metric: VectorMetric,
    pub normalize: bool,
    pub max_input: usize,
    pub query_template: String,
    pub document_template: String,
    pub batch_limit: usize,
    pub capabilities: EmbeddingCapabilities,
}

impl EmbeddingModelProfile {
    pub fn validate_dense_v1(&self) -> Result<(), String> {
        if self.schema_version != EMBEDDING_MODEL_PROFILE_SCHEMA_VERSION {
            return Err("unsupported embedding model profile schema".into());
        }
        if self.id.is_empty()
            || self.provider.is_empty()
            || self.model_id.is_empty()
            || self.model_revision.is_empty()
            || self.dimension == 0
            || self.max_input == 0
            || self.batch_limit == 0
        {
            return Err("embedding identity and positive limits are required".into());
        }
        if !self.capabilities.dense || self.capabilities.sparse || self.capabilities.multi_vector {
            return Err("only dense single-vector profiles are supported".into());
        }
        Ok(())
    }
}

pub fn bge_m3_dense_v1() -> EmbeddingModelProfile {
    EmbeddingModelProfile {
        schema_version: EMBEDDING_MODEL_PROFILE_SCHEMA_VERSION,
        id: "bge_m3_dense_v1".into(),
        version: 1,
        provider: "ollama".into(),
        model_id: "bge-m3:latest".into(),
        model_revision: "provider_reported".into(),
        dimension: 1024,
        metric: VectorMetric::Cosine,
        normalize: true,
        max_input: 8192,
        query_template: "{text}".into(),
        document_template: "{text}".into(),
        batch_limit: 32,
        capabilities: EmbeddingCapabilities {
            dense: true,
            sparse: false,
            multi_vector: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_is_valid_and_not_storage_specific() {
        let profile = bge_m3_dense_v1();
        assert!(profile.validate_dense_v1().is_ok());
        assert_eq!(profile.dimension, 1024);
    }
}
