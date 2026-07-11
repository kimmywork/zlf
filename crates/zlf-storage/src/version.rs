use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use zlf_core::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVersion {
    pub version_id: u64,
    pub properties: std::collections::HashMap<String, Value>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}
