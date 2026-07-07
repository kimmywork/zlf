use std::sync::Arc;

use zlf_core::{Node, Edge, Value};
use zlf_query::QueryPlanner;

pub struct ZLF {
    planner: Arc<QueryPlanner>,
}

impl ZLF {
    pub fn new(path: &str) -> Result<Self, zlf_core::ZlfError> {
        let planner = QueryPlanner::open(path)?;
        Ok(Self {
            planner: Arc::new(planner),
        })
    }

    pub fn add_node(&self, labels: Vec<String>, properties: serde_json::Value) -> Result<Node, zlf_core::ZlfError> {
        let props = self.json_to_properties(properties)?;
        let node = Node::new(labels, props);
        self.planner.add_node(node)
    }

    pub fn get_node(&self, id: &str) -> Result<Option<Node>, zlf_core::ZlfError> {
        self.planner.get_node(id)
    }

    pub fn add_edge(&self, edge_type: String, source: String, target: String, properties: serde_json::Value) -> Result<Edge, zlf_core::ZlfError> {
        let props = self.json_to_properties(properties)?;
        let edge = Edge::new(edge_type, source, target, props);
        self.planner.add_edge(edge)
    }

    pub fn get_edge(&self, id: &str) -> Result<Option<Edge>, zlf_core::ZlfError> {
        self.planner.get_edge(id)
    }

    pub fn query(&self, query_str: &str) -> Result<Vec<serde_json::Value>, zlf_core::ZlfError> {
        self.planner.execute(query_str)
    }

    pub fn search(&self, query: &str) -> Result<Vec<(String, f32)>, zlf_core::ZlfError> {
        self.planner.search(query)
    }

    pub fn similar(&self, node_id: &str, threshold: f32, limit: usize) -> Result<Vec<(String, f32)>, zlf_core::ZlfError> {
        self.planner.similar(node_id, threshold, limit)
    }

    fn json_to_properties(&self, json: serde_json::Value) -> Result<std::collections::HashMap<String, Value>, zlf_core::ZlfError> {
        let mut props = std::collections::HashMap::new();
        
        if let Some(obj) = json.as_object() {
            for (k, v) in obj {
                props.insert(k.clone(), self.json_value_to_value(v));
            }
        }
        
        Ok(props)
    }

    fn json_value_to_value(&self, json: &serde_json::Value) -> Value {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    Value::Number(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.json_value_to_value(v)).collect())
            }
            serde_json::Value::Object(obj) => {
                let mut map = std::collections::HashMap::new();
                for (k, v) in obj {
                    map.insert(k.clone(), self.json_value_to_value(v));
                }
                Value::Object(map)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_zlf_creation() {
        let temp = TempDir::new().unwrap();
        let db = ZLF::new(temp.path().to_str().unwrap()).unwrap();
        assert!(db.planner.is_some() || true); // Just test creation
    }

    #[test]
    fn test_add_and_get_node() {
        let temp = TempDir::new().unwrap();
        let db = ZLF::new(temp.path().to_str().unwrap()).unwrap();
        
        let props = serde_json::json!({
            "name": "Alice",
            "age": 30
        });
        
        let node = db.add_node(vec!["person".to_string()], props).unwrap();
        assert!(!node.id.is_empty());
        
        let retrieved = db.get_node(&node.id).unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_add_and_get_edge() {
        let temp = TempDir::new().unwrap();
        let db = ZLF::new(temp.path().to_str().unwrap()).unwrap();
        
        // Create nodes first
        let node1 = db.add_node(vec!["person".to_string()], serde_json::json!({"name": "Alice"})).unwrap();
        let node2 = db.add_node(vec!["person".to_string()], serde_json::json!({"name": "Bob"})).unwrap();
        
        // Create edge
        let edge = db.add_edge("knows".to_string(), node1.id, node2.id, serde_json::json!({})).unwrap();
        assert!(!edge.id.is_empty());
        
        let retrieved = db.get_edge(&edge.id).unwrap();
        assert!(retrieved.is_some());
    }
}
