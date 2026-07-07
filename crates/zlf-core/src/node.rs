use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    pub labels: Vec<String>,
    pub properties: HashMap<String, Value>,
    pub current_version: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Node {
    pub fn new(labels: Vec<String>, properties: HashMap<String, Value>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            labels,
            properties,
            current_version: 1,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_id(id: String, labels: Vec<String>, properties: HashMap<String, Value>) -> Self {
        let now = Utc::now();
        Self {
            id,
            labels,
            properties,
            current_version: 1,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|l| l == label)
    }

    pub fn get_property(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }

    pub fn set_property(&mut self, key: String, value: Value) {
        self.properties.insert(key, value);
        self.updated_at = Utc::now();
    }

    pub fn remove_property(&mut self, key: &str) -> Option<Value> {
        let result = self.properties.remove(key);
        if result.is_some() {
            self.updated_at = Utc::now();
        }
        result
    }

    pub fn increment_version(&mut self) {
        self.current_version += 1;
        self.updated_at = Utc::now();
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new(vec![], HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let mut props = HashMap::new();
        props.insert("name".to_string(), Value::String("Alice".to_string()));
        props.insert("age".to_string(), Value::Number(30.0));

        let node = Node::new(vec!["person".to_string()], props.clone());

        assert!(!node.id.is_empty());
        assert_eq!(node.labels, vec!["person"]);
        assert_eq!(node.properties, props);
        assert_eq!(node.current_version, 1);
    }

    #[test]
    fn test_node_with_id() {
        let node = Node::with_id(
            "test-id".to_string(),
            vec!["person".to_string()],
            HashMap::new(),
        );

        assert_eq!(node.id, "test-id");
    }

    #[test]
    fn test_has_label() {
        let node = Node::new(
            vec!["person".to_string(), "employee".to_string()],
            HashMap::new(),
        );

        assert!(node.has_label("person"));
        assert!(node.has_label("employee"));
        assert!(!node.has_label("company"));
    }

    #[test]
    fn test_get_set_property() {
        let mut node = Node::new(vec![], HashMap::new());

        node.set_property(
            "name".to_string(),
            Value::String("Alice".to_string()),
        );
        assert_eq!(
            node.get_property("name"),
            Some(&Value::String("Alice".to_string()))
        );

        node.remove_property("name");
        assert_eq!(node.get_property("name"), None);
    }

    #[test]
    fn test_increment_version() {
        let mut node = Node::new(vec![], HashMap::new());
        assert_eq!(node.current_version, 1);

        node.increment_version();
        assert_eq!(node.current_version, 2);

        node.increment_version();
        assert_eq!(node.current_version, 3);
    }

    #[test]
    fn test_empty_labels_array() {
        let node = Node::new(vec![], HashMap::new());
        assert!(node.labels.is_empty());
    }

    #[test]
    fn test_empty_properties_object() {
        let node = Node::new(vec!["person".to_string()], HashMap::new());
        assert!(node.properties.is_empty());
    }

    #[test]
    fn test_nested_properties() {
        let mut nested = HashMap::new();
        nested.insert("inner".to_string(), Value::String("value".to_string()));
        
        let mut props = HashMap::new();
        props.insert("nested".to_string(), Value::Object(nested));
        
        let node = Node::new(vec![], props.clone());
        assert_eq!(node.properties, props);
    }

    #[test]
    fn test_large_properties() {
        // Test with a large string value (>1KB)
        let large_string = "x".repeat(2000);
        let mut props = HashMap::new();
        props.insert("large".to_string(), Value::String(large_string.clone()));
        
        let node = Node::new(vec![], props);
        assert_eq!(
            node.properties.get("large"),
            Some(&Value::String(large_string))
        );
    }

    #[test]
    fn test_node_id_max_length() {
        // Test node ID with 255 characters (max allowed)
        let long_id = "a".repeat(255);
        let node = Node::with_id(long_id.clone(), vec![], HashMap::new());
        assert_eq!(node.id.len(), 255);
    }
}
