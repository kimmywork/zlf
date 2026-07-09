use zlf_core::{Edge, Node, Result, ZlfError};

use crate::Storage;

impl Storage {
    pub(crate) fn update_node_indexes(&self, node: &Node) -> Result<()> {
        // Index by label
        for label in &node.labels {
            let key = format!("idx:label:{}:{}", label, node.id);
            self.db
                .put(&key, [])
                .map_err(|e| ZlfError::Internal(e.to_string()))?;
        }

        Ok(())
    }

    pub(crate) fn remove_node_indexes(&self, node: &Node) -> Result<()> {
        // Remove label indexes
        for label in &node.labels {
            let key = format!("idx:label:{}:{}", label, node.id);
            self.db
                .delete(&key)
                .map_err(|e| ZlfError::Internal(e.to_string()))?;
        }

        Ok(())
    }

    pub(crate) fn update_edge_indexes(&self, edge: &Edge) -> Result<()> {
        // Index by edge type
        let key = format!("idx:edge_type:{}:{}", edge.edge_type, edge.id);
        self.db
            .put(&key, [])
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    pub(crate) fn remove_edge_indexes(&self, edge: &Edge) -> Result<()> {
        // Remove edge type index
        let key = format!("idx:edge_type:{}:{}", edge.edge_type, edge.id);
        self.db
            .delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }
}
