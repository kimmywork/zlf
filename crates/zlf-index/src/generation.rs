use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const GENERATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GenerationId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationState {
    Draft,
    Building,
    Validating,
    Active,
    Retired,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationMetadata {
    pub schema_version: u32,
    pub id: GenerationId,
    pub target: String,
    pub profile_name: String,
    pub profile_version: u32,
    pub backend_schema: String,
    pub source_snapshot_sequence: u64,
    pub state: GenerationState,
    pub document_count: u64,
    pub checksum: Option<String>,
    pub created_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexStatus {
    pub target: String,
    pub active_generation: Option<GenerationId>,
    pub scanned_watermark: u64,
    pub published_watermark: u64,
    pub state: Option<GenerationState>,
    pub document_count: u64,
}
