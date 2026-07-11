use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rocksdb::{Options, DB};
use zlf_core::{Edge, Node, Result, Value, ZlfError};

mod bulk;
mod bulk_session;
mod canonical;
mod delete;
mod graph_query;
mod lifecycle;
mod memory;
mod mutation;
mod outbox;
mod property;
mod query;
mod raw;
mod version;

pub use bulk::{StorageRecord, StorageRecordPlan, STORAGE_KEY_VERSION};
pub use bulk_session::{BulkSession, BulkSessionState};
pub use lifecycle::{
    EntityResolution, EntityState, MutationEvent, MutationKind, MutationReceipt, MutationSequence,
    MUTATION_EVENT_SCHEMA_VERSION,
};
pub use raw::RawMutation;
pub use version::NodeVersion;

pub struct Storage {
    pub(crate) db: Arc<DB>,
    write_lock: Mutex<()>,
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
        let storage = Self {
            db: Arc::new(db),
            write_lock: Mutex::new(()),
        };
        storage.initialize_lifecycle(false)?;
        Ok(storage)
    }

    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ZlfError::FileNotFound(path.display().to_string()));
        }

        let opts = Options::default();
        let db = DB::open(&opts, path)
            .map_err(|e| ZlfError::Internal(format!("Failed to open database: {}", e)))?;
        let storage = Self {
            db: Arc::new(db),
            write_lock: Mutex::new(()),
        };
        storage.initialize_lifecycle(true)?;
        Ok(storage)
    }

    pub fn create_node(&self, node: Node) -> Result<Node> {
        if node.id.len() > 255 {
            return Err(ZlfError::NodeIdTooLong);
        }
        let _guard = self.write_guard()?;
        if self.get_node(&node.id)?.is_some() {
            return Err(ZlfError::NodeAlreadyExists(node.id));
        }
        self.commit_node_upsert(None, &node, all_node_fields(&node))?;
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
        let _guard = self.write_guard()?;
        let mut node = self
            .get_node(id)?
            .ok_or_else(|| ZlfError::NodeNotFound(id.to_string()))?;
        if labels.iter().all(|label| node.labels.contains(label)) {
            return Ok(node);
        }
        let old = node.clone();
        for label in labels {
            if !node.labels.contains(label) {
                node.labels.push(label.clone());
            }
        }
        node.increment_version();
        self.commit_node_upsert(Some(&old), &node, BTreeSet::from(["labels".into()]))?;
        Ok(node)
    }

    pub fn update_node(
        &self,
        id: &str,
        properties: std::collections::HashMap<String, Value>,
    ) -> Result<Node> {
        let _guard = self.write_guard()?;
        let mut node = self
            .get_node(id)?
            .ok_or_else(|| ZlfError::NodeNotFound(id.to_string()))?;
        let old = node.clone();
        let changed_fields = old
            .properties
            .keys()
            .chain(properties.keys())
            .cloned()
            .collect();
        node.properties = properties;
        node.increment_version();
        self.commit_node_upsert(Some(&old), &node, changed_fields)?;
        Ok(node)
    }

    pub fn delete_node(&self, id: &str) -> Result<bool> {
        let _guard = self.write_guard()?;
        let node = self
            .get_node(id)?
            .ok_or_else(|| ZlfError::NodeNotFound(id.to_string()))?;
        let edges = self
            .get_all_edges()?
            .into_iter()
            .filter(|edge| edge.source == id || edge.target == id)
            .collect::<Vec<_>>();
        self.commit_node_cascade_delete(&node, &edges)?;
        Ok(true)
    }

    pub fn create_edge(&self, edge: Edge) -> Result<Edge> {
        if edge.edge_type.is_empty() {
            return Err(ZlfError::EmptyEdgeType);
        }
        let _guard = self.write_guard()?;
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

        self.commit_edge_upsert(None, &edge, all_edge_fields(&edge))?;
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
        let _guard = self.write_guard()?;
        let edge = self
            .get_edge(id)?
            .ok_or_else(|| ZlfError::EdgeNotFound(id.to_string()))?;
        self.commit_edge_delete(&edge)?;
        Ok(true)
    }

    pub(crate) fn write_guard(&self) -> Result<MutexGuard<'_, ()>> {
        self.write_lock
            .lock()
            .map_err(|error| ZlfError::Internal(error.to_string()))
    }
}

fn all_node_fields(node: &Node) -> BTreeSet<String> {
    let mut fields = node.properties.keys().cloned().collect::<BTreeSet<_>>();
    fields.insert("labels".into());
    fields
}

fn all_edge_fields(edge: &Edge) -> BTreeSet<String> {
    let mut fields = edge.properties.keys().cloned().collect::<BTreeSet<_>>();
    fields.insert("relation".into());
    fields
}
