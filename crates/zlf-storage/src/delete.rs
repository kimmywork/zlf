use std::collections::HashMap;

use zlf_core::{Node, Result, Value, ZlfError};

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
        let _guard = self.write_guard()?;
        let node = match self.get_node(id)? {
            Some(node) => node,
            None => return Ok(false),
        };
        let edges = self
            .get_all_edges()?
            .into_iter()
            .filter(|edge| edge.source == id || edge.target == id)
            .collect::<Vec<_>>();
        self.commit_node_cascade_delete(&node, &edges)?;
        Ok(true)
    }

    /// Remove a label from an existing node.
    /// If the node does not have the label, this is a no-op.
    pub fn remove_node_label(&self, id: &str, label: &str) -> Result<bool> {
        let _guard = self.write_guard()?;
        let mut node = self
            .get_node(id)?
            .ok_or_else(|| ZlfError::NodeNotFound(id.to_string()))?;
        let Some(index) = node.labels.iter().position(|current| current == label) else {
            return Ok(false);
        };
        let old = node.clone();
        node.labels.remove(index);
        node.increment_version();
        self.commit_node_upsert(
            Some(&old),
            &node,
            std::collections::BTreeSet::from(["labels".into()]),
        )?;
        Ok(true)
    }

    /// Delete a property key from a node.
    /// Returns true if the property existed and was removed.
    pub fn delete_node_property(&self, id: &str, key: &str) -> Result<bool> {
        let _guard = self.write_guard()?;
        let mut node = self
            .get_node(id)?
            .ok_or_else(|| ZlfError::NodeNotFound(id.to_string()))?;
        if !node.properties.contains_key(key) {
            return Ok(false);
        }
        let old = node.clone();
        node.properties.remove(key);
        node.increment_version();
        self.commit_node_upsert(
            Some(&old),
            &node,
            std::collections::BTreeSet::from([key.to_string()]),
        )?;
        Ok(true)
    }

    /// Append a single property value to a node without replacing others.
    /// Used by idempotent fact writing.
    pub fn insert_node_property(&self, id: &str, key: &str, value: Value) -> Result<()> {
        let _guard = self.write_guard()?;
        let existing = self.get_node(id)?;
        let mut node = existing
            .clone()
            .unwrap_or_else(|| Node::with_id(id.to_string(), Vec::new(), HashMap::new()));
        node.properties.insert(key.to_string(), value);
        node.increment_version();
        self.commit_node_upsert(
            existing.as_ref(),
            &node,
            std::collections::BTreeSet::from([key.to_string()]),
        )?;
        Ok(())
    }
}
