use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use serde_json::Value as JsonValue;
use zlf_core::{Node, Value};
use zlf_index::{
    Bm25FieldOptions, ChunkingProfile, EmbeddingModelProfile, EntityMatcher, FieldIndexOptions,
    IndexProfileArtifact, TemporalRole, VectorFieldOptions, INDEX_PROFILE_SCHEMA_VERSION,
};
use zlf_query::{BatchEmbeddingProvider, EmbeddingProviderFailure};

#[allow(clippy::too_many_lines)]
pub fn profile() -> IndexProfileArtifact {
    let mut profile = IndexProfileArtifact {
        schema_version: INDEX_PROFILE_SCHEMA_VERSION,
        name: "enterprise".into(),
        version: 1,
        source_hash: String::new(),
        matcher: EntityMatcher::NodeLabels {
            labels: vec!["document".into()],
        },
        fields: BTreeMap::from([
            (
                "body".into(),
                FieldIndexOptions {
                    bm25: Some(Bm25FieldOptions {
                        analyzer_id: "unicode_jieba_v1".into(),
                        language: Some("en".into()),
                        analyzer_version: 1,
                        weight: 1.0,
                        k1: 1.2,
                        b: 0.75,
                    }),
                    vector: Some(VectorFieldOptions {
                        model_profile: "bge_m3_dense_v1".into(),
                        chunking: ChunkingProfile::WholeField { version: 1 },
                    }),
                    temporal: None,
                },
            ),
            ("valid_from".into(), temporal_field(TemporalRole::ValidFrom)),
            ("valid_to".into(), temporal_field(TemporalRole::ValidTo)),
        ]),
        created_at: Utc::now(),
    };
    profile.refresh_source_hash();
    profile
}

fn temporal_field(role: TemporalRole) -> FieldIndexOptions {
    FieldIndexOptions {
        bm25: None,
        vector: None,
        temporal: Some(role),
    }
}

pub fn node(row: &JsonValue) -> Node {
    Node::with_id(
        row["_id"].as_str().unwrap().into(),
        vec!["document".into()],
        HashMap::from([
            (
                "access_group".into(),
                Value::String(row["access_group"].as_str().unwrap().into()),
            ),
            (
                "active".into(),
                Value::Bool(row["active"].as_bool().unwrap()),
            ),
            (
                "body".into(),
                Value::String(row["body"].as_str().unwrap().into()),
            ),
            (
                "valid_from".into(),
                Value::String(row["valid_from"].as_str().unwrap().into()),
            ),
            (
                "valid_to".into(),
                Value::String(row["valid_to"].as_str().unwrap().into()),
            ),
        ]),
    )
}

pub struct DeterministicProvider;

#[async_trait::async_trait]
impl BatchEmbeddingProvider for DeterministicProvider {
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

pub struct FailOnceProvider(pub AtomicBool);

impl FailOnceProvider {
    pub fn new() -> Self {
        Self(AtomicBool::new(true))
    }
}

#[async_trait::async_trait]
impl BatchEmbeddingProvider for FailOnceProvider {
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
        if self.0.swap(false, Ordering::SeqCst) {
            return Err(EmbeddingProviderFailure {
                class: "injected_retry".into(),
                retryable: true,
            });
        }
        Ok(texts.iter().map(|text| vector(text)).collect())
    }
}

fn vector(text: &str) -> Vec<f32> {
    let mut values = vec![0.0; 1024];
    let topic = (0..64)
        .find(|index| text.contains(&format!("topic{index:02}")))
        .unwrap_or(64);
    values[topic] = 1.0;
    values
}
