use std::path::Path;
use std::sync::Arc;

use rocksdb::{Options, DB};
use zlf_core::{Edge, Node, Result, Value, ZlfError};

mod bulk;
mod canonical;
mod delete;
mod graph_query;
mod indexes;
mod memory;
mod query;
mod raw;
mod version;

pub use bulk::{StorageRecord, StorageRecordPlan, STORAGE_KEY_VERSION};
pub use raw::RawMutation;
pub use version::NodeVersion;

pub struct Storage {
    pub(crate) db: Arc<DB>,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if path.exists() {
            return Err(ZlfError::DatabaseAlreadyExists(path.display().to_string()));
        }

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        let db = DB::open(&opts, path)
            .map_err(|e| ZlfError::Internal(format!("Failed to open database: {}", e)))?;

        Ok(Self { db: Arc::new(db) })
    }

    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ZlfError::FileNotFound(path.display().to_string()));
        }

        let opts = Options::default();
        let db = DB::open(&opts, path)
            .map_err(|e| ZlfError::Internal(format!("Failed to open database: {}", e)))?;

        Ok(Self { db: Arc::new(db) })
    }

    pub fn create_node(&self, node: Node) -> Result<Node> {
        // Validate node ID length
        if node.id.len() > 255 {
            return Err(ZlfError::NodeIdTooLong);
        }

        // Check if node already exists
        let key = format!("node:{}", node.id);
        if self
            .db
            .get(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?
            .is_some()
        {
            return Err(ZlfError::NodeAlreadyExists(node.id));
        }

        let plan = Self::compile_node_records(&node)?;
        self.write_record_plans([&plan])?;
        Ok(node)
    }

    pub fn get_node(&self, id: &str) -> Result<Option<Node>> {
        let key = format!("node:{}", id);

        match self
            .db
            .get(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?
        {
            Some(data) => {
                let node: Node = bincode::deserialize(&data)
                    .map_err(|e| ZlfError::Serialization(e.to_string()))?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    pub fn add_labels(&self, id: &str, labels: &[String]) -> Result<Node> {
        let mut node = self
            .get_node(id)?
            .ok_or_else(|| ZlfError::NodeNotFound(id.to_string()))?;
        if labels.iter().all(|label| node.labels.contains(label)) {
            return Ok(node);
        }
        self.remove_node_indexes(&node)?;
        for label in labels {
            if !node.labels.contains(label) {
                node.labels.push(label.clone());
            }
        }
        node.increment_version();
        let key = format!("node:{}", node.id);
        let data = bincode::serialize(&node).map_err(|e| ZlfError::Serialization(e.to_string()))?;
        self.db
            .put(&key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        self.create_version(&node)?;
        self.update_node_indexes(&node)?;
        Ok(node)
    }

    pub fn update_node(
        &self,
        id: &str,
        properties: std::collections::HashMap<String, Value>,
    ) -> Result<Node> {
        let mut node = self
            .get_node(id)?
            .ok_or_else(|| ZlfError::NodeNotFound(id.to_string()))?;

        // Remove old indexes
        self.remove_node_indexes(&node)?;

        // Update properties
        node.properties = properties;
        node.increment_version();

        // Serialize and store
        let key = format!("node:{}", node.id);
        let data = bincode::serialize(&node).map_err(|e| ZlfError::Serialization(e.to_string()))?;

        self.db
            .put(&key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        // Store new version
        self.create_version(&node)?;

        // Update indexes
        self.update_node_indexes(&node)?;

        Ok(node)
    }

    pub fn delete_node(&self, id: &str) -> Result<bool> {
        let node = self
            .get_node(id)?
            .ok_or_else(|| ZlfError::NodeNotFound(id.to_string()))?;

        // Remove indexes
        self.remove_node_indexes(&node)?;

        // Delete node
        let key = format!("node:{}", id);
        self.db
            .delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        // Delete versions
        self.delete_versions(id)?;

        Ok(true)
    }

    pub fn create_edge(&self, edge: Edge) -> Result<Edge> {
        // Validate edge type
        if edge.edge_type.is_empty() {
            return Err(ZlfError::EmptyEdgeType);
        }

        // Check if source node exists
        if self.get_node(&edge.source)?.is_none() {
            return Err(ZlfError::SourceNodeNotFound(edge.source));
        }

        // Check if target node exists
        if self.get_node(&edge.target)?.is_none() {
            return Err(ZlfError::TargetNodeNotFound(edge.target));
        }

        // Check if edge already exists
        let key = format!("edge:{}", edge.id);
        if self
            .db
            .get(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?
            .is_some()
        {
            return Err(ZlfError::EdgeAlreadyExists(edge.id));
        }

        let plan = Self::compile_edge_records(&edge)?;
        self.write_record_plans([&plan])?;
        Ok(edge)
    }

    pub fn get_edge(&self, id: &str) -> Result<Option<Edge>> {
        let key = format!("edge:{}", id);

        match self
            .db
            .get(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?
        {
            Some(data) => {
                let edge: Edge = bincode::deserialize(&data)
                    .map_err(|e| ZlfError::Serialization(e.to_string()))?;
                Ok(Some(edge))
            }
            None => Ok(None),
        }
    }

    pub fn delete_edge(&self, id: &str) -> Result<bool> {
        let edge = self
            .get_edge(id)?
            .ok_or_else(|| ZlfError::EdgeNotFound(id.to_string()))?;

        // Remove indexes
        self.remove_edge_indexes(&edge)?;

        // Delete edge
        let key = format!("edge:{}", id);
        self.db
            .delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(true)
    }
}
