use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use zlf_core::{Node, Result, Value, ZlfError};

use crate::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVersion {
    pub version_id: u64,
    pub properties: std::collections::HashMap<String, Value>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}

impl Storage {
    pub(crate) fn create_version(&self, node: &Node) -> Result<()> {
        let version = NodeVersion {
            version_id: node.current_version,
            properties: node.properties.clone(),
            valid_from: node.updated_at,
            valid_to: None,
        };

        let key = format!("ver:{}:{}", node.id, node.current_version);
        let data =
            bincode::serialize(&version).map_err(|e| ZlfError::Serialization(e.to_string()))?;

        self.db
            .put(&key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    pub(crate) fn delete_versions(&self, node_id: &str) -> Result<()> {
        let prefix = format!("ver:{}:", node_id);
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);

        for item in iter {
            let (key, _) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);
            if key_str.starts_with(&prefix) {
                self.db
                    .delete(&key)
                    .map_err(|e| ZlfError::Internal(e.to_string()))?;
            }
        }

        Ok(())
    }
}
