use std::path::Path;
use std::sync::Arc;

use zlf_core::{Node, Edge, ZlfError, Result};
use zlf_storage::Storage;
use zlf_index::{TemporalIndex, BM25Index, VectorIndex};
use zlf_prolog::{PrologParser, Term, Query};

pub struct QueryPlanner {
    storage: Arc<Storage>,
    temporal_index: Arc<TemporalIndex>,
    bm25_index: Arc<BM25Index>,
    vector_index: Arc<VectorIndex>,
}

impl QueryPlanner {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        
        let storage = Arc::new(Storage::open(path.join("storage"))?);
        let temporal_index = Arc::new(TemporalIndex::open(path.join("temporal"))?);
        let bm25_index = Arc::new(BM25Index::open(path.join("bm25"))?);
        let vector_index = Arc::new(VectorIndex::open(path.join("vector"))?);
        
        Ok(Self {
            storage,
            temporal_index,
            bm25_index,
            vector_index,
        })
    }

    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        
        let storage = Arc::new(Storage::open_existing(path.join("storage"))?);
        let temporal_index = Arc::new(TemporalIndex::open(path.join("temporal"))?);
        let bm25_index = Arc::new(BM25Index::open(path.join("bm25"))?);
        let vector_index = Arc::new(VectorIndex::open(path.join("vector"))?);
        
        Ok(Self {
            storage,
            temporal_index,
            bm25_index,
            vector_index,
        })
    }

    pub fn execute(&self, query_str: &str) -> Result<Vec<serde_json::Value>> {
        // Parse the query
        let query = PrologParser::parse_query(query_str)?;
        
        // Execute based on query type
        match query {
            Query::Goal(term) => {
                self.execute_goal(&term)
            }
            Query::RuleDef(_rule) => {
                // Store rule for later use
                Ok(vec![])
            }
        }
    }

    fn execute_goal(&self, term: &Term) -> Result<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        
        // Extract predicate name and arguments
        if let Some((name, args)) = term.as_compound() {
            match name {
                "node" => {
                    results.extend(self.query_nodes(args)?);
                }
                "edge" => {
                    results.extend(self.query_edges(args)?);
                }
                "search" => {
                    results.extend(self.query_search(args)?);
                }
                "similar_to" => {
                    results.extend(self.query_similar(args)?);
                }
                "time_range" => {
                    results.extend(self.query_time_range(args)?);
                }
                "before" => {
                    results.extend(self.query_before(args)?);
                }
                "after" => {
                    results.extend(self.query_after(args)?);
                }
                _ => {
                    // Try to match with stored rules
                    // For now, return empty
                }
            }
        }
        
        Ok(results)
    }

    fn query_nodes(&self, _args: &[Term]) -> Result<Vec<serde_json::Value>> {
        let results = Vec::new();
        
        // Get all nodes and filter
        // This is a simplified implementation
        // In production, we would use indexes
        
        Ok(results)
    }

    fn query_edges(&self, _args: &[Term]) -> Result<Vec<serde_json::Value>> {
        let results = Vec::new();
        
        // Get all edges and filter
        // This is a simplified implementation
        
        Ok(results)
    }

    fn query_search(&self, args: &[Term]) -> Result<Vec<serde_json::Value>> {
        if args.len() < 2 {
            return Err(ZlfError::SyntaxError(0, "search requires 2 arguments".to_string()));
        }
        
        let query = match &args[1] {
            Term::String(s) => s.clone(),
            _ => return Err(ZlfError::SyntaxError(0, "search query must be a string".to_string())),
        };
        
        let search_results = self.bm25_index.search(&query)?;
        
        let mut results = Vec::new();
        for (node_id, score) in search_results {
            let mut result = serde_json::Map::new();
            result.insert("node_id".to_string(), serde_json::Value::String(node_id));
            result.insert("score".to_string(), serde_json::json!(score as f64));
            results.push(serde_json::Value::Object(result));
        }
        
        Ok(results)
    }

    fn query_similar(&self, args: &[Term]) -> Result<Vec<serde_json::Value>> {
        if args.len() < 2 {
            return Err(ZlfError::SyntaxError(0, "similar_to requires at least 2 arguments".to_string()));
        }
        
        // Get the node ID from first argument
        let node_id = match &args[0] {
            Term::Atom(s) => s.clone(),
            Term::String(s) => s.clone(),
            _ => return Err(ZlfError::SyntaxError(0, "first argument must be a node ID".to_string())),
        };
        
        // Get threshold from second argument
        let threshold = match &args[1] {
            Term::Number(n) => *n as f32,
            _ => 0.8, // Default threshold
        };
        
        // Get embedding for the node
        if let Some(entry) = self.vector_index.get_entry(&node_id)? {
            let similar = self.vector_index.find_similar(&entry.embedding, threshold, 10)?;
            
            let mut results = Vec::new();
            for (id, score) in similar {
                let mut result = serde_json::Map::new();
                result.insert("node_id".to_string(), serde_json::Value::String(id));
                result.insert("similarity".to_string(), serde_json::json!(score as f64));
                results.push(serde_json::Value::Object(result));
            }
            
            Ok(results)
        } else {
            Err(ZlfError::NoEmbedding(node_id))
        }
    }

    fn query_time_range(&self, _args: &[Term]) -> Result<Vec<serde_json::Value>> {
        // Simplified implementation
        Ok(vec![])
    }

    fn query_before(&self, _args: &[Term]) -> Result<Vec<serde_json::Value>> {
        // Simplified implementation
        Ok(vec![])
    }

    fn query_after(&self, _args: &[Term]) -> Result<Vec<serde_json::Value>> {
        // Simplified implementation
        Ok(vec![])
    }

    pub fn add_node(&self, node: Node) -> Result<Node> {
        self.storage.create_node(node)
    }

    pub fn get_node(&self, id: &str) -> Result<Option<Node>> {
        self.storage.get_node(id)
    }

    pub fn add_edge(&self, edge: Edge) -> Result<Edge> {
        self.storage.create_edge(edge)
    }

    pub fn get_edge(&self, id: &str) -> Result<Option<Edge>> {
        self.storage.get_edge(id)
    }

    pub fn search(&self, query: &str) -> Result<Vec<(String, f32)>> {
        self.bm25_index.search(query)
    }

    pub fn similar(&self, node_id: &str, threshold: f32, limit: usize) -> Result<Vec<(String, f32)>> {
        if let Some(entry) = self.vector_index.get_entry(node_id)? {
            self.vector_index.find_similar(&entry.embedding, threshold, limit)
        } else {
            Err(ZlfError::NoEmbedding(node_id.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::collections::HashMap;
    use zlf_core::Value;

    fn create_test_planner() -> (QueryPlanner, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let planner = QueryPlanner::open(temp_dir.path().join("db")).unwrap();
        (planner, temp_dir)
    }

    #[test]
    fn test_create_planner() {
        let (_planner, _temp) = create_test_planner();
    }

    #[test]
    fn test_add_and_get_node() {
        let (planner, _temp) = create_test_planner();
        
        let mut props = HashMap::new();
        props.insert("name".to_string(), Value::String("Alice".to_string()));
        
        let node = Node::new(vec!["person".to_string()], props);
        let created = planner.add_node(node).unwrap();
        
        let retrieved = planner.get_node(&created.id).unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_add_and_get_edge() {
        let (planner, _temp) = create_test_planner();
        
        let mut props = HashMap::new();
        props.insert("name".to_string(), Value::String("Alice".to_string()));
        
        let node1 = Node::with_id("alice".to_string(), vec!["person".to_string()], props.clone());
        let node2 = Node::with_id("bob".to_string(), vec!["person".to_string()], props);
        
        planner.add_node(node1).unwrap();
        planner.add_node(node2).unwrap();
        
        let edge = Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        );
        
        let created = planner.add_edge(edge).unwrap();
        let retrieved = planner.get_edge(&created.id).unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_search() {
        let (planner, _temp) = create_test_planner();
        
        // Add some searchable data
        // For now, search returns empty
        let results = planner.search("test").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_invalid_query_syntax() {
        let (planner, _temp) = create_test_planner();
        
        let result = planner.execute("invalid query syntax");
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_predicate() {
        let (planner, _temp) = create_test_planner();
        
        let result = planner.execute("?unsupported(alice).");
        // Should return empty results, not error
        assert!(result.is_ok());
    }
}
