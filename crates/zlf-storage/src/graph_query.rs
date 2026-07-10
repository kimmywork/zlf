use zlf_core::{Edge, Result};

use crate::Storage;

/// Edge adjacency queries using out/in indexes.
impl Storage {
    /// Get all outgoing edges from a source node.
    /// Optionally filtered by edge type.
    pub fn get_outgoing_edges(&self, source: &str, edge_type: Option<&str>) -> Result<Vec<Edge>> {
        let prefix = match edge_type {
            Some(et) => format!("idx:edge_out:{source}:{et}:"),
            None => format!("idx:edge_out:{source}:"),
        };
        let keys = self.scan_prefix(&prefix)?;
        let mut edges = Vec::new();
        for (_, value) in &keys {
            let edge_id = String::from_utf8_lossy(value);
            if let Some(edge) = self.get_edge(&edge_id)? {
                edges.push(edge);
            }
        }
        Ok(edges)
    }

    /// Get all incoming edges to a target node.
    /// Optionally filtered by edge type.
    pub fn get_incoming_edges(&self, target: &str, edge_type: Option<&str>) -> Result<Vec<Edge>> {
        let prefix = match edge_type {
            Some(et) => format!("idx:edge_in:{target}:{et}:"),
            None => format!("idx:edge_in:{target}:"),
        };
        let keys = self.scan_prefix(&prefix)?;
        let mut edges = Vec::new();
        for (_, value) in &keys {
            let edge_id = String::from_utf8_lossy(value);
            if let Some(edge) = self.get_edge(&edge_id)? {
                edges.push(edge);
            }
        }
        Ok(edges)
    }

    /// Get the set of distinct outgoing neighbor node IDs from a source.
    pub fn get_outgoing_neighbors(&self, source: &str) -> Result<Vec<String>> {
        let prefix = format!("idx:edge_out:{source}:");
        let keys = self.scan_prefix(&prefix)?;
        let mut neighbors: Vec<String> = Vec::new();
        for (key, _) in &keys {
            let suffix = key
                .strip_prefix(&format!("idx:edge_out:{source}:"))
                .unwrap_or("");
            let target = suffix.split(':').nth(1).unwrap_or("");
            if !target.is_empty() && !neighbors.contains(&target.to_string()) {
                neighbors.push(target.to_string());
            }
        }
        Ok(neighbors)
    }

    /// Count outgoing edges from a source node.
    pub fn count_outgoing_edges(&self, source: &str) -> Result<usize> {
        let prefix = format!("idx:edge_out:{source}:");
        Ok(self.scan_prefix(&prefix)?.len())
    }

    /// Count incoming edges to a target node.
    pub fn count_incoming_edges(&self, target: &str) -> Result<usize> {
        let prefix = format!("idx:edge_in:{target}:");
        Ok(self.scan_prefix(&prefix)?.len())
    }
}
