use tempfile::TempDir;

use zlf_core::{Node, Edge, Value};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let mut props = std::collections::HashMap::new();
        props.insert("name".to_string(), Value::String("Alice".to_string()));
        
        let node = Node::new(vec!["person".to_string()], props);
        assert!(!node.id.is_empty());
        assert_eq!(node.labels, vec!["person"]);
    }

    #[test]
    fn test_edge_creation() {
        let edge = Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            std::collections::HashMap::new(),
        );
        
        assert!(!edge.id.is_empty());
        assert_eq!(edge.edge_type, "knows");
        assert_eq!(edge.source, "alice");
        assert_eq!(edge.target, "bob");
    }
}
