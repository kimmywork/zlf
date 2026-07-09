use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};

use zlf_core::{Edge, Node, Result, Value, ZlfError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVersion {
    pub version_id: u64,
    pub properties: std::collections::HashMap<String, Value>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}

pub struct Storage {
    db: Arc<DB>,
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

        // Serialize and store
        let data = bincode::serialize(&node).map_err(|e| ZlfError::Serialization(e.to_string()))?;

        self.db
            .put(&key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        // Store version
        self.create_version(&node)?;

        // Update indexes
        self.update_node_indexes(&node)?;

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

        // Serialize and store
        let data = bincode::serialize(&edge).map_err(|e| ZlfError::Serialization(e.to_string()))?;

        self.db
            .put(&key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        // Update indexes
        self.update_edge_indexes(&edge)?;

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

    fn create_version(&self, node: &Node) -> Result<()> {
        let version = NodeVersion {
            version_id: node.current_version,
            properties: node.properties.clone(),
            valid_from: node.updated_at,
            valid_to: None,
        };

        let key = format!("ver:{}:{}", node.id, node.current_version);
        let data =
            bincode::serialize(&version).map_err(|e| ZlfError::Serialization(e.to_string()))?;

        self.db
            .put(&key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    fn delete_versions(&self, node_id: &str) -> Result<()> {
        let prefix = format!("ver:{}:", node_id);
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);

        for item in iter {
            let (key, _) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);
            if key_str.starts_with(&prefix) {
                self.db
                    .delete(&key)
                    .map_err(|e| ZlfError::Internal(e.to_string()))?;
            }
        }

        Ok(())
    }

    fn update_node_indexes(&self, node: &Node) -> Result<()> {
        // Index by label
        for label in &node.labels {
            let key = format!("idx:label:{}:{}", label, node.id);
            self.db
                .put(&key, [])
                .map_err(|e| ZlfError::Internal(e.to_string()))?;
        }

        Ok(())
    }

    fn remove_node_indexes(&self, node: &Node) -> Result<()> {
        // Remove label indexes
        for label in &node.labels {
            let key = format!("idx:label:{}:{}", label, node.id);
            self.db
                .delete(&key)
                .map_err(|e| ZlfError::Internal(e.to_string()))?;
        }

        Ok(())
    }

    fn update_edge_indexes(&self, edge: &Edge) -> Result<()> {
        // Index by edge type
        let key = format!("idx:edge_type:{}:{}", edge.edge_type, edge.id);
        self.db
            .put(&key, [])
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    fn remove_edge_indexes(&self, edge: &Edge) -> Result<()> {
        // Remove edge type index
        let key = format!("idx:edge_type:{}:{}", edge.edge_type, edge.id);
        self.db
            .delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

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

    pub fn create_memory(
        &self,
        id: &str,
        memory_type: &str,
        content: std::collections::HashMap<String, Value>,
        importance: f32,
    ) -> Result<Node> {
        let mut props = std::collections::HashMap::new();
        props.insert(
            "memory_type".to_string(),
            Value::String(memory_type.to_string()),
        );
        props.insert("content".to_string(), Value::Object(content));
        props.insert("importance".to_string(), Value::Number(importance as f64));
        props.insert(
            "created_at".to_string(),
            Value::String(Utc::now().to_rfc3339()),
        );

        let node = Node::with_id(
            id.to_string(),
            vec!["memory".to_string(), memory_type.to_string()],
            props,
        );

        self.create_node(node)
    }

    pub fn get_memory(&self, id: &str) -> Result<Option<Node>> {
        self.get_node(id)
    }

    pub fn query_memories_by_type(&self, memory_type: &str) -> Result<Vec<Node>> {
        self.get_nodes_by_label(memory_type)
    }

    pub fn expire_memories(&self, older_than: DateTime<Utc>) -> Result<usize> {
        let mut expired = 0;

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with("node:") {
                let node: Node = bincode::deserialize(&value)
                    .map_err(|e| ZlfError::Serialization(e.to_string()))?;

                if node.labels.contains(&"memory".to_string()) {
                    if let Some(Value::String(created_at_str)) = node.properties.get("created_at") {
                        if let Ok(created_time) = DateTime::parse_from_rfc3339(created_at_str) {
                            if created_time.with_timezone(&Utc) < older_than {
                                self.delete_node(&node.id)?;
                                expired += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(expired)
    }

    pub fn put_raw(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db
            .put(key, value)
            .map_err(|e| ZlfError::Internal(e.to_string()))
    }

    pub fn delete_raw(&self, key: &str) -> Result<()> {
        self.db
            .delete(key)
            .map_err(|e| ZlfError::Internal(e.to_string()))
    }

    pub fn scan_prefix(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let mut results = Vec::new();
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);
            if key_str.starts_with(prefix) {
                results.push((key_str.to_string(), value.to_vec()));
            }
        }
        Ok(results)
    }

    pub fn close(&self) {
        // DB is closed when dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open(temp_dir.path().join("test.db")).unwrap();
        (storage, temp_dir)
    }

    fn create_test_node(id: &str) -> Node {
        let mut props = HashMap::new();
        props.insert("name".to_string(), Value::String("Alice".to_string()));
        Node::with_id(id.to_string(), vec!["person".to_string()], props)
    }

    #[test]
    fn test_create_and_get_node() {
        let (storage, _temp) = create_test_storage();
        let node = create_test_node("alice");

        let created = storage.create_node(node.clone()).unwrap();
        assert_eq!(created.id, "alice");

        let retrieved = storage.get_node("alice").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "alice");
    }

    #[test]
    fn test_duplicate_node_id() {
        let (storage, _temp) = create_test_storage();
        let node = create_test_node("alice");

        storage.create_node(node.clone()).unwrap();

        let result = storage.create_node(node);
        assert!(matches!(result, Err(ZlfError::NodeAlreadyExists(_))));
    }

    #[test]
    fn test_node_not_found() {
        let (storage, _temp) = create_test_storage();

        let result = storage.get_node("nonexistent");
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_update_node() {
        let (storage, _temp) = create_test_storage();
        let node = create_test_node("alice");

        storage.create_node(node).unwrap();

        let mut new_props = HashMap::new();
        new_props.insert(
            "name".to_string(),
            Value::String("Alice Updated".to_string()),
        );

        let updated = storage.update_node("alice", new_props).unwrap();
        assert_eq!(updated.current_version, 2);
        assert_eq!(
            updated.properties.get("name"),
            Some(&Value::String("Alice Updated".to_string()))
        );
    }

    #[test]
    fn test_delete_node() {
        let (storage, _temp) = create_test_storage();
        let node = create_test_node("alice");

        storage.create_node(node).unwrap();

        let deleted = storage.delete_node("alice").unwrap();
        assert!(deleted);

        let retrieved = storage.get_node("alice").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_create_edge() {
        let (storage, _temp) = create_test_storage();

        let node1 = create_test_node("alice");
        let node2 = create_test_node("bob");

        storage.create_node(node1).unwrap();
        storage.create_node(node2).unwrap();

        let mut props = HashMap::new();
        props.insert("since".to_string(), Value::Number(2020.0));

        let edge = Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            props,
        );

        let created = storage.create_edge(edge).unwrap();
        assert_eq!(created.edge_type, "knows");
        assert_eq!(created.source, "alice");
        assert_eq!(created.target, "bob");
    }

    #[test]
    fn test_empty_edge_type() {
        let (storage, _temp) = create_test_storage();

        let node1 = create_test_node("alice");
        let node2 = create_test_node("bob");

        storage.create_node(node1).unwrap();
        storage.create_node(node2).unwrap();

        let edge = Edge::new(
            String::new(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        );

        let result = storage.create_edge(edge);
        assert!(matches!(result, Err(ZlfError::EmptyEdgeType)));
    }

    #[test]
    fn test_source_node_not_found() {
        let (storage, _temp) = create_test_storage();

        let node2 = create_test_node("bob");
        storage.create_node(node2).unwrap();

        let edge = Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        );

        let result = storage.create_edge(edge);
        assert!(matches!(result, Err(ZlfError::SourceNodeNotFound(_))));
    }

    #[test]
    fn test_target_node_not_found() {
        let (storage, _temp) = create_test_storage();

        let node1 = create_test_node("alice");
        storage.create_node(node1).unwrap();

        let edge = Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        );

        let result = storage.create_edge(edge);
        assert!(matches!(result, Err(ZlfError::TargetNodeNotFound(_))));
    }

    #[test]
    fn test_get_nodes_by_label() {
        let (storage, _temp) = create_test_storage();

        let node1 = create_test_node("alice");
        let node2 = create_test_node("bob");
        let node3 = Node::with_id(
            "acme".to_string(),
            vec!["company".to_string()],
            HashMap::new(),
        );

        storage.create_node(node1).unwrap();
        storage.create_node(node2).unwrap();
        storage.create_node(node3).unwrap();

        let persons = storage.get_nodes_by_label("person").unwrap();
        assert_eq!(persons.len(), 2);

        let companies = storage.get_nodes_by_label("company").unwrap();
        assert_eq!(companies.len(), 1);
    }

    #[test]
    fn test_get_all_edges() {
        let (storage, _temp) = create_test_storage();
        storage.create_node(create_test_node("alice")).unwrap();
        storage.create_node(create_test_node("bob")).unwrap();
        storage.create_node(create_test_node("charlie")).unwrap();

        storage
            .create_edge(Edge::new(
                "knows".to_string(),
                "alice".to_string(),
                "bob".to_string(),
                HashMap::new(),
            ))
            .unwrap();
        storage
            .create_edge(Edge::new(
                "knows".to_string(),
                "bob".to_string(),
                "charlie".to_string(),
                HashMap::new(),
            ))
            .unwrap();

        let edges = storage.get_all_edges().unwrap();
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_update_with_same_properties_creates_version() {
        let (storage, _temp) = create_test_storage();
        let node = create_test_node("alice");

        storage.create_node(node).unwrap();

        let mut props = HashMap::new();
        props.insert("name".to_string(), Value::String("Alice".to_string()));

        let updated = storage.update_node("alice", props).unwrap();
        assert_eq!(updated.current_version, 2);
    }

    #[test]
    fn test_get_node_versions() {
        let (storage, _temp) = create_test_storage();
        let node = create_test_node("alice");

        storage.create_node(node).unwrap();

        let mut props = HashMap::new();
        props.insert(
            "name".to_string(),
            Value::String("Alice Updated".to_string()),
        );

        storage.update_node("alice", props).unwrap();

        let versions = storage.get_node_versions("alice").unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version_id, 1);
        assert_eq!(versions[1].version_id, 2);
    }

    #[test]
    fn test_get_node_at_time() {
        let (storage, _temp) = create_test_storage();
        let node = create_test_node("alice");

        storage.create_node(node).unwrap();

        let now = Utc::now();
        let node_at_time = storage.get_node_at_time("alice", now).unwrap();
        assert!(node_at_time.is_some());
    }

    #[test]
    fn test_create_and_get_memory() {
        let (storage, _temp) = create_test_storage();

        let mut content = HashMap::new();
        content.insert("message".to_string(), Value::String("Hello".to_string()));

        let memory = storage
            .create_memory("mem1", "conversation", content, 0.8)
            .unwrap();
        assert!(memory.labels.contains(&"memory".to_string()));
        assert!(memory.labels.contains(&"conversation".to_string()));

        let retrieved = storage.get_memory("mem1").unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_query_memories_by_type() {
        let (storage, _temp) = create_test_storage();

        let mut content1 = HashMap::new();
        content1.insert("message".to_string(), Value::String("Hello".to_string()));

        let mut content2 = HashMap::new();
        content2.insert("message".to_string(), Value::String("World".to_string()));

        storage
            .create_memory("mem1", "conversation", content1, 0.8)
            .unwrap();
        storage
            .create_memory("mem2", "knowledge", content2, 0.9)
            .unwrap();

        let conversations = storage.query_memories_by_type("conversation").unwrap();
        assert_eq!(conversations.len(), 1);

        let knowledge = storage.query_memories_by_type("knowledge").unwrap();
        assert_eq!(knowledge.len(), 1);
    }
}
