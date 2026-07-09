use chrono::{DateTime, Utc};

use zlf_core::{Node, Result, Value, ZlfError};

use crate::Storage;

impl Storage {
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
}
