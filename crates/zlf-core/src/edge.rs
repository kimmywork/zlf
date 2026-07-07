use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub id: String,
    pub edge_type: String,
    pub source: String,
    pub target: String,
    pub properties: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Edge {
    pub fn new(
        edge_type: String,
        source: String,
        target: String,
        properties: HashMap<String, Value>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            edge_type,
            source,
            target,
            properties,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_id(
        id: String,
        edge_type: String,
        source: String,
        target: String,
        properties: HashMap<String, Value>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            edge_type,
            source,
            target,
            properties,
            created_at: now,
            updated_at: now,
        }
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

    pub fn is_self_referencing(&self) -> bool {
        self.source == self.target
    }
}

impl Default for Edge {
    fn default() -> Self {
        Self::new(
            String::new(),
            String::new(),
            String::new(),
            HashMap::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_creation() {
        let mut props = HashMap::new();
        props.insert("since".to_string(), Value::Number(2020.0));

        let edge = Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            props.clone(),
        );

        assert!(!edge.id.is_empty());
        assert_eq!(edge.edge_type, "knows");
        assert_eq!(edge.source, "alice");
        assert_eq!(edge.target, "bob");
        assert_eq!(edge.properties, props);
    }

    #[test]
    fn test_edge_with_id() {
        let edge = Edge::with_id(
            "test-id".to_string(),
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        );

        assert_eq!(edge.id, "test-id");
    }

    #[test]
    fn test_self_referencing_edge() {
        let edge = Edge::new(
            "self_loop".to_string(),
            "node1".to_string(),
            "node1".to_string(),
            HashMap::new(),
        );

        assert!(edge.is_self_referencing());

        let edge2 = Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        );

        assert!(!edge2.is_self_referencing());
    }

    #[test]
    fn test_get_set_property() {
        let mut edge = Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        );

        edge.set_property("since".to_string(), Value::Number(2020.0));
        assert_eq!(
            edge.get_property("since"),
            Some(&Value::Number(2020.0))
        );

        edge.remove_property("since");
        assert_eq!(edge.get_property("since"), None);
    }

    #[test]
    fn test_edge_with_no_properties() {
        let edge = Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        );

        assert!(edge.properties.is_empty());
    }

    #[test]
    fn test_empty_edge_type() {
        let edge = Edge::new(
            String::new(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        );

        assert!(edge.edge_type.is_empty());
    }
}
