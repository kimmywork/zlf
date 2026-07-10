use std::collections::HashMap;

use zlf_core::{Edge, Node, Result, Value, ZlfError};

use crate::Storage;

impl Storage {
    /// Delete an edge by its triple (source:type:target).
    /// Also removes all index entries for the edge.
    /// Returns true if the edge existed and was deleted.
    pub fn delete_edge_by_triple(
        &self,
        source: &str,
        edge_type: &str,
        target: &str,
    ) -> Result<bool> {
        let edges = self.get_outgoing_edges(source, Some(edge_type))?;
        let mut deleted = false;
        for edge in edges.into_iter().filter(|edge| edge.target == target) {
            self.delete_edge(&edge.id)?;
            deleted = true;
        }
        if deleted {
            Ok(true)
        } else {
            let edge_id = format!("{source}:{edge_type}:{target}");
            self.delete_edge(&edge_id).or(Ok(false))
        }
    }

    /// Delete a node and all incident edges (cascade).
    /// Removes all labels, properties, and index entries.
    /// Returns true if the node existed and was deleted.
    pub fn delete_node_cascade(&self, id: &str) -> Result<bool> {
        let node = match self.get_node(id)? {
            Some(n) => n,
            None => return Ok(false),
        };

        // Find and delete all incident edges
        let edges = self.scan_prefix("edge:")?;
        for (key, value) in edges {
            let edge: Edge =
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))?;
            if edge.source == id || edge.target == id {
                self.remove_edge_indexes(&edge)?;
                self.db
                    .delete(&key)
                    .map_err(|e| ZlfError::Internal(e.to_string()))?;
            }
        }

        // Remove all label indexes
        self.remove_node_indexes(&node)?;

        // Delete versions
        self.delete_versions(id)?;

        // Delete node record
        let key = format!("node:{id}");
        self.db
            .delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(true)
    }

    /// Remove a label from an existing node.
    /// If the node does not have the label, this is a no-op.
    pub fn remove_node_label(&self, id: &str, label: &str) -> Result<bool> {
        let mut node = match self.get_node(id)? {
            Some(n) => n,
            None => return Err(ZlfError::NodeNotFound(id.to_string())),
        };

        let pos = node.labels.iter().position(|l| l == label);
        match pos {
            Some(idx) => {
                // Remove label index
                let idx_key = format!("idx:label:{label}:{id}");
                self.db
                    .delete(&idx_key)
                    .map_err(|e| ZlfError::Internal(e.to_string()))?;

                // Update node
                node.labels.remove(idx);
                node.increment_version();
                let node_key = format!("node:{id}");
                let data = bincode::serialize(&node)
                    .map_err(|e| ZlfError::Serialization(e.to_string()))?;
                self.db
                    .put(&node_key, data)
                    .map_err(|e| ZlfError::Internal(e.to_string()))?;
                self.create_version(&node)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Delete a property key from a node.
    /// Returns true if the property existed and was removed.
    pub fn delete_node_property(&self, id: &str, key: &str) -> Result<bool> {
        let mut node = match self.get_node(id)? {
            Some(n) => n,
            None => return Err(ZlfError::NodeNotFound(id.to_string())),
        };

        if node.properties.contains_key(key) {
            node.properties.remove(key);
            node.increment_version();
            let node_key = format!("node:{id}");
            let data =
                bincode::serialize(&node).map_err(|e| ZlfError::Serialization(e.to_string()))?;
            self.db
                .put(&node_key, data)
                .map_err(|e| ZlfError::Internal(e.to_string()))?;
            self.create_version(&node)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Append a single property value to a node without replacing others.
    /// Used by idempotent fact writing.
    pub fn insert_node_property(&self, id: &str, key: &str, value: Value) -> Result<()> {
        let mut node = match self.get_node(id)? {
            Some(n) => n,
            None => Node::with_id(id.to_string(), Vec::new(), HashMap::new()),
        };
        node.properties.insert(key.to_string(), value);
        node.increment_version();
        let node_key = format!("node:{id}");
        let data = bincode::serialize(&node).map_err(|e| ZlfError::Serialization(e.to_string()))?;
        self.db
            .put(&node_key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        self.create_version(&node)?;
        Ok(())
    }
}
