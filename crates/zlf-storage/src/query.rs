use chrono::{DateTime, Utc};

use zlf_core::{Edge, Node, Result, ZlfError};

use crate::bulk::property_index_key;
use crate::{NodeVersion, Storage};

impl Storage {
    pub fn get_nodes_by_label(&self, label: &str) -> Result<Vec<Node>> {
        let prefix = format!("idx:label:{}:", label);
        let mut nodes = Vec::new();

        for (key, _) in self.scan_prefix(&prefix)? {
            let node_id = &key[prefix.len()..];
            if let Some(node) = self.get_node(node_id)? {
                nodes.push(node);
            }
        }

        Ok(nodes)
    }

    pub fn get_nodes_by_property(&self, key: &str, value: &zlf_core::Value) -> Result<Vec<Node>> {
        let prefix = property_index_key(key, value)?;
        let mut nodes = Vec::new();
        for (record_key, _) in self.scan_prefix(&prefix)? {
            if let Some(node) = self.get_node(&record_key[prefix.len()..])? {
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    pub fn get_all_nodes(&self) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();

        for (_, value) in self.scan_prefix("node:")? {
            let node: Node =
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    pub fn get_edges_by_type(&self, edge_type: &str) -> Result<Vec<Edge>> {
        let prefix = format!("idx:edge_type:{}:", edge_type);
        let mut edges = Vec::new();

        for (key, _) in self.scan_prefix(&prefix)? {
            let edge_id = &key[prefix.len()..];
            if let Some(edge) = self.get_edge(edge_id)? {
                edges.push(edge);
            }
        }

        Ok(edges)
    }

    pub fn get_all_edges(&self) -> Result<Vec<Edge>> {
        let mut edges = Vec::new();

        for (_, value) in self.scan_prefix("edge:")? {
            let edge: Edge =
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))?;
            edges.push(edge);
        }

        Ok(edges)
    }

    pub fn get_node_versions(&self, node_id: &str) -> Result<Vec<NodeVersion>> {
        let prefix = format!("ver:{}:", node_id);
        let mut versions = Vec::new();

        for (_, value) in self.scan_prefix(&prefix)? {
            let version: NodeVersion =
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))?;
            versions.push(version);
        }

        versions.sort_by_key(|version| version.version_id);

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
