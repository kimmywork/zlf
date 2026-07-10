use zlf_core::{Edge, Node, Result, ZlfError};

use crate::bulk::{predicate_metadata_key, property_index_key};
use crate::Storage;

impl Storage {
    pub(crate) fn update_node_indexes(&self, node: &Node) -> Result<()> {
        for label in &node.labels {
            let key = format!("idx:label:{}:{}", label, node.id);
            self.db
                .put(&key, [])
                .map_err(|e| ZlfError::Internal(e.to_string()))?;
            self.db
                .put(predicate_metadata_key("label", label), label.as_bytes())
                .map_err(|error| ZlfError::Internal(error.to_string()))?;
        }
        for (key, value) in &node.properties {
            self.db
                .put(predicate_metadata_key("property", key), key.as_bytes())
                .map_err(|error| ZlfError::Internal(error.to_string()))?;
            if matches!(
                value,
                zlf_core::Value::Array(_) | zlf_core::Value::Object(_)
            ) {
                continue;
            }
            self.db
                .put(
                    format!("{}{}", property_index_key(key, value)?, node.id),
                    [],
                )
                .map_err(|error| ZlfError::Internal(error.to_string()))?;
        }
        Ok(())
    }

    pub(crate) fn remove_node_indexes(&self, node: &Node) -> Result<()> {
        for label in &node.labels {
            let key = format!("idx:label:{}:{}", label, node.id);
            self.db
                .delete(&key)
                .map_err(|e| ZlfError::Internal(e.to_string()))?;
        }
        for (key, value) in &node.properties {
            if matches!(
                value,
                zlf_core::Value::Array(_) | zlf_core::Value::Object(_)
            ) {
                continue;
            }
            self.db
                .delete(format!("{}{}", property_index_key(key, value)?, node.id))
                .map_err(|error| ZlfError::Internal(error.to_string()))?;
        }
        Ok(())
    }

    pub(crate) fn remove_edge_indexes(&self, edge: &Edge) -> Result<()> {
        let key = format!("idx:edge_type:{}:{}", edge.edge_type, edge.id);
        self.db
            .delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        let out_key = format!(
            "idx:edge_out:{}:{}:{}",
            edge.source, edge.edge_type, edge.target
        );
        self.db
            .delete(&out_key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        let in_key = format!(
            "idx:edge_in:{}:{}:{}",
            edge.target, edge.edge_type, edge.source
        );
        self.db
            .delete(&in_key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        Ok(())
    }
}
