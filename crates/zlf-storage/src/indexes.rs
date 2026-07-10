use zlf_core::{Edge, Node, Result, ZlfError};

use crate::Storage;

impl Storage {
    pub(crate) fn update_node_indexes(&self, node: &Node) -> Result<()> {
        for label in &node.labels {
            let key = format!("idx:label:{}:{}", label, node.id);
            self.db
                .put(&key, [])
                .map_err(|e| ZlfError::Internal(e.to_string()))?;
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
        Ok(())
    }

    pub(crate) fn update_edge_indexes(&self, edge: &Edge) -> Result<()> {
        let key = format!("idx:edge_type:{}:{}", edge.edge_type, edge.id);
        self.db
            .put(&key, [])
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        // Outgoing edge index: source -> (type, target)
        let out_key = format!(
            "idx:edge_out:{}:{}:{}",
            edge.source, edge.edge_type, edge.target
        );
        self.db
            .put(&out_key, edge.id.as_bytes())
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        // Incoming edge index: target -> (type, source)
        let in_key = format!(
            "idx:edge_in:{}:{}:{}",
            edge.target, edge.edge_type, edge.source
        );
        self.db
            .put(&in_key, edge.id.as_bytes())
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
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
