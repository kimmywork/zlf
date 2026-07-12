use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::lexical::{
    TANTIVY_BM25_B, TANTIVY_BM25_K1, UNICODE_JIEBA_ANALYZER_ID, UNICODE_JIEBA_ANALYZER_VERSION,
};

pub const INDEX_PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityMatcher {
    NodeLabels { labels: Vec<String> },
    EdgeTypes { edge_types: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkingProfile {
    Explicit {
        version: u32,
    },
    WholeField {
        version: u32,
    },
    ParagraphHeading {
        version: u32,
    },
    FixedTokenWindow {
        version: u32,
        size: u32,
        overlap: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bm25FieldOptions {
    pub analyzer_id: String,
    pub analyzer_version: u32,
    pub weight: f32,
    pub k1: f32,
    pub b: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorFieldOptions {
    pub model_profile: String,
    pub chunking: ChunkingProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalRole {
    Event,
    ValidFrom,
    ValidTo,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FieldIndexOptions {
    pub bm25: Option<Bm25FieldOptions>,
    pub vector: Option<VectorFieldOptions>,
    pub temporal: Option<TemporalRole>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexProfileArtifact {
    pub schema_version: u32,
    pub name: String,
    pub version: u32,
    pub source_hash: String,
    pub matcher: EntityMatcher,
    pub fields: BTreeMap<String, FieldIndexOptions>,
    pub created_at: DateTime<Utc>,
}

impl IndexProfileArtifact {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != INDEX_PROFILE_SCHEMA_VERSION {
            return Err("unsupported index profile schema".into());
        }
        if self.name.is_empty() || self.source_hash.is_empty() || self.fields.is_empty() {
            return Err("profile name, source hash, and fields are required".into());
        }
        if self.source_hash != self.computed_source_hash() {
            return Err("profile source hash does not match canonical content".into());
        }
        match &self.matcher {
            EntityMatcher::NodeLabels { labels } if labels.is_empty() => {
                return Err("node matcher requires labels".into());
            }
            EntityMatcher::EdgeTypes { edge_types } if edge_types.is_empty() => {
                return Err("edge matcher requires edge types".into());
            }
            _ => {}
        }
        for (field, options) in &self.fields {
            validate_field(field, options)?;
        }
        Ok(())
    }

    pub fn computed_source_hash(&self) -> String {
        let canonical = (
            self.schema_version,
            &self.name,
            self.version,
            &self.matcher,
            &self.fields,
        );
        let bytes = bincode::serialize(&canonical).expect("profile contract is serializable");
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub fn refresh_source_hash(&mut self) {
        self.source_hash = self.computed_source_hash();
    }
}

fn validate_field(field: &str, options: &FieldIndexOptions) -> Result<(), String> {
    if field.is_empty() {
        return Err("field name cannot be empty".into());
    }
    if options.bm25.is_none() && options.vector.is_none() && options.temporal.is_none() {
        return Err(format!("field {field} has no index options"));
    }
    if let Some(bm25) = &options.bm25 {
        if bm25.analyzer_id != UNICODE_JIEBA_ANALYZER_ID
            || bm25.analyzer_version != UNICODE_JIEBA_ANALYZER_VERSION
            || bm25.k1 != TANTIVY_BM25_K1
            || bm25.b != TANTIVY_BM25_B
            || !bm25.weight.is_finite()
            || bm25.weight <= 0.0
            || !bm25.k1.is_finite()
            || bm25.k1 <= 0.0
            || !bm25.b.is_finite()
            || !(0.0..=1.0).contains(&bm25.b)
        {
            return Err(format!("invalid BM25 options for field {field}"));
        }
    }
    if let Some(vector) = &options.vector {
        if vector.model_profile.is_empty() || !valid_chunking(&vector.chunking) {
            return Err(format!("invalid vector options for field {field}"));
        }
    }
    Ok(())
}

fn valid_chunking(profile: &ChunkingProfile) -> bool {
    match profile {
        ChunkingProfile::Explicit { version }
        | ChunkingProfile::WholeField { version }
        | ChunkingProfile::ParagraphHeading { version } => *version > 0,
        ChunkingProfile::FixedTokenWindow {
            version,
            size,
            overlap,
        } => *version > 0 && *size > 0 && overlap < size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_tantivy_bm25_parameters_are_rejected() {
        let mut options = FieldIndexOptions {
            bm25: Some(Bm25FieldOptions {
                analyzer_id: UNICODE_JIEBA_ANALYZER_ID.into(),
                analyzer_version: UNICODE_JIEBA_ANALYZER_VERSION,
                weight: 1.0,
                k1: TANTIVY_BM25_K1,
                b: TANTIVY_BM25_B,
            }),
            vector: None,
            temporal: None,
        };
        assert!(validate_field("body", &options).is_ok());
        options.bm25.as_mut().unwrap().k1 = 2.0;
        assert!(validate_field("body", &options).is_err());
        options.bm25.as_mut().unwrap().k1 = TANTIVY_BM25_K1;
        options.bm25.as_mut().unwrap().analyzer_id = "unsupported".into();
        assert!(validate_field("body", &options).is_err());
    }

    #[test]
    fn fixed_window_configuration_round_trips() {
        let profile = ChunkingProfile::FixedTokenWindow {
            version: 1,
            size: 128,
            overlap: 16,
        };
        let bytes = bincode::serialize(&profile).unwrap();
        let decoded: ChunkingProfile = bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded, profile);
    }
}
