use chrono::{DateTime, Utc};

use zlf_core::{Edge, Node, Result, ZlfError};

use crate::{NodeVersion, Storage};

impl Storage {
    pub fn get_nodes_by_label(&self, label: &str) -> Result<Vec<Node>> {
        let prefix = format!("idx:label:{}:", label);
        let mut nodes = Vec::new();

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, _) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with(&prefix) {
                let node_id = &key_str[prefix.len()..];
                if let Some(node) = self.get_node(node_id)? {
                    nodes.push(node);
                }
            }
        }

        Ok(nodes)
    }

    pub fn get_all_nodes(&self) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);

            // Only get node keys (not index keys)
            if key_str.starts_with("node:") {
                let node: Node = bincode::deserialize(&value)
                    .map_err(|e| ZlfError::Serialization(e.to_string()))?;
                nodes.push(node);
            }
        }

        Ok(nodes)
    }

    pub fn get_edges_by_type(&self, edge_type: &str) -> Result<Vec<Edge>> {
        let prefix = format!("idx:edge_type:{}:", edge_type);
        let mut edges = Vec::new();

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, _) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with(&prefix) {
                let edge_id = &key_str[prefix.len()..];
                if let Some(edge) = self.get_edge(edge_id)? {
                    edges.push(edge);
                }
            }
        }

        Ok(edges)
    }

    pub fn get_all_edges(&self) -> Result<Vec<Edge>> {
        let mut edges = Vec::new();

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with("edge:") {
                let edge: Edge = bincode::deserialize(&value)
                    .map_err(|e| ZlfError::Serialization(e.to_string()))?;
                edges.push(edge);
            }
        }

        Ok(edges)
    }

    pub fn get_node_versions(&self, node_id: &str) -> Result<Vec<NodeVersion>> {
        let prefix = format!("ver:{}:", node_id);
        let mut versions = Vec::new();

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with(&prefix) {
                let version: NodeVersion = bincode::deserialize(&value)
                    .map_err(|e| ZlfError::Serialization(e.to_string()))?;
                versions.push(version);
            }
        }

        versions.sort_by(|a, b| a.version_id.cmp(&b.version_id));

        Ok(versions)
    }

    pub fn get_node_at_time(
        &self,
        node_id: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<Option<Node>> {
        let versions = self.get_node_versions(node_id)?;

        for version in versions.iter().rev() {
            if version.valid_from <= timestamp
                && (version.valid_to.is_none() || version.valid_to.unwrap() > timestamp)
            {
                // Found the version at this time
                let mut node = self.get_node(node_id)?.unwrap_or_default();
                node.properties = version.properties.clone();
                node.current_version = version.version_id;
                return Ok(Some(node));
            }
        }

        Ok(None)
    }
}
