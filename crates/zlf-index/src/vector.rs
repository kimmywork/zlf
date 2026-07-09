use std::path::Path;
use std::sync::Arc;

use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};

use zlf_core::{Result, ZlfError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub node_id: String,
    pub embedding: Vec<f32>,
    pub model: String,
}

pub struct VectorIndex {
    db: Arc<DB>,
}

impl VectorIndex {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let mut opts = Options::default();
        opts.create_if_missing(true);

        let db = DB::open(&opts, path)
            .map_err(|e| ZlfError::Internal(format!("Failed to open vector index: {}", e)))?;

        Ok(Self { db: Arc::new(db) })
    }

    pub fn add_entry(&self, entry: VectorEntry) -> Result<()> {
        let key = format!("vector:{}", entry.node_id);

        let data =
            bincode::serialize(&entry).map_err(|e| ZlfError::Serialization(e.to_string()))?;

        self.db
            .put(&key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    pub fn get_entry(&self, node_id: &str) -> Result<Option<VectorEntry>> {
        let key = format!("vector:{}", node_id);

        match self
            .db
            .get(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?
        {
            Some(data) => {
                let entry: VectorEntry = bincode::deserialize(&data)
                    .map_err(|e| ZlfError::Serialization(e.to_string()))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    pub fn find_similar(
        &self,
        query_embedding: &[f32],
        threshold: f32,
        limit: usize,
    ) -> Result<Vec<(String, f32)>> {
        let mut results = Vec::new();

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (_, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;

            let entry: VectorEntry =
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))?;

            // Check dimension match
            if entry.embedding.len() != query_embedding.len() {
                continue;
            }

            let similarity = cosine_similarity(query_embedding, &entry.embedding);
            if similarity >= threshold {
                results.push((entry.node_id, similarity));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    pub fn remove_entry(&self, node_id: &str) -> Result<()> {
        let key = format!("vector:{}", node_id);

        self.db
            .delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_index() -> (VectorIndex, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let index = VectorIndex::open(temp_dir.path().join("vector")).unwrap();
        (index, temp_dir)
    }

    #[test]
    fn test_add_and_get_entry() {
        let (index, _temp) = create_test_index();

        index
            .add_entry(VectorEntry {
                node_id: "alice".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
                model: "test".to_string(),
            })
            .unwrap();

        let entry = index.get_entry("alice").unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().embedding, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_find_similar() {
        let (index, _temp) = create_test_index();

        index
            .add_entry(VectorEntry {
                node_id: "alice".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
                model: "test".to_string(),
            })
            .unwrap();

        index
            .add_entry(VectorEntry {
                node_id: "bob".to_string(),
                embedding: vec![0.15, 0.25, 0.35],
                model: "test".to_string(),
            })
            .unwrap();

        index
            .add_entry(VectorEntry {
                node_id: "acme".to_string(),
                embedding: vec![0.9, 0.8, 0.7],
                model: "test".to_string(),
            })
            .unwrap();

        // Query similar to alice
        let query = vec![0.1, 0.2, 0.3];
        let results = index.find_similar(&query, 0.0, 10).unwrap();

        // Debug: print results
        for (id, score) in &results {
            println!("{}: {}", id, score);
        }

        // Should find all nodes
        assert!(!results.is_empty());

        // First result should be alice (exact match)
        assert_eq!(results[0].0, "alice");
    }

    #[test]
    fn test_remove_entry() {
        let (index, _temp) = create_test_index();

        index
            .add_entry(VectorEntry {
                node_id: "alice".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
                model: "test".to_string(),
            })
            .unwrap();

        index.remove_entry("alice").unwrap();

        let entry = index.get_entry("alice").unwrap();
        assert!(entry.is_none());
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_find_similar_with_no_embeddings() {
        let (index, _temp) = create_test_index();

        let query = vec![0.1, 0.2, 0.3];
        let results = index.find_similar(&query, 0.8, 10).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_dimension_mismatch() {
        let (index, _temp) = create_test_index();

        index
            .add_entry(VectorEntry {
                node_id: "alice".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
                model: "test".to_string(),
            })
            .unwrap();

        // Query with different dimension
        let query = vec![0.1, 0.2];
        let results = index.find_similar(&query, 0.8, 10).unwrap();

        // Should not find alice due to dimension mismatch
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_similar_with_threshold_zero() {
        let (index, _temp) = create_test_index();

        index
            .add_entry(VectorEntry {
                node_id: "alice".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
                model: "test".to_string(),
            })
            .unwrap();

        index
            .add_entry(VectorEntry {
                node_id: "bob".to_string(),
                embedding: vec![0.9, 0.8, 0.7],
                model: "test".to_string(),
            })
            .unwrap();

        // Query with threshold 0.0 should return all
        let query = vec![0.1, 0.2, 0.3];
        let results = index.find_similar(&query, 0.0, 10).unwrap();

        // Should return both nodes
        assert_eq!(results.len(), 2);
    }
}
