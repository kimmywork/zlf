use std::path::Path;
use std::sync::Arc;
use std::collections::HashMap;

use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};
use jieba_rs::Jieba;

use zlf_core::{ZlfError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BM25Entry {
    pub node_id: String,
    pub token: String,
    pub score: f32,
}

pub struct BM25Index {
    db: Arc<DB>,
    jieba: Jieba,
}

impl BM25Index {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        let db = DB::open(&opts, path)
            .map_err(|e| ZlfError::Internal(format!("Failed to open BM25 index: {}", e)))?;

        Ok(Self {
            db: Arc::new(db),
            jieba: Jieba::new(),
        })
    }

    pub fn tokenize(&self, text: &str) -> Vec<String> {
        // Use jieba for Chinese tokenization
        let words = self.jieba.cut(text, false);
        words.iter()
            .map(|w| w.word.to_lowercase())
            .filter(|w| !w.is_empty())
            .collect()
    }

    pub fn add_entry(&self, entry: BM25Entry) -> Result<()> {
        let key = format!("bm25:{}:{}", entry.token, entry.node_id);
        
        let data = bincode::serialize(&entry)
            .map_err(|e| ZlfError::Serialization(e.to_string()))?;
        
        self.db.put(&key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    pub fn index_text(&self, node_id: &str, text: &str) -> Result<()> {
        let tokens = self.tokenize(text);
        let mut token_counts: HashMap<String, f32> = HashMap::new();
        
        for token in &tokens {
            *token_counts.entry(token.clone()).or_insert(0.0) += 1.0;
        }
        
        // Calculate TF-IDF (simplified: just TF for now)
        for (token, count) in &token_counts {
            self.add_entry(BM25Entry {
                node_id: node_id.to_string(),
                token: token.clone(),
                score: *count,
            })?;
        }
        
        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<(String, f32)>> {
        let tokens = self.tokenize(query);
        let mut scores: HashMap<String, f32> = HashMap::new();
        
        for token in &tokens {
            let prefix = format!("bm25:{}:", token);
            let iter = self.db.iterator(rocksdb::IteratorMode::Start);
            
            for item in iter {
                let (key, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
                let key_str = String::from_utf8_lossy(&key);
                
                if key_str.starts_with(&prefix) {
                    let entry: BM25Entry = bincode::deserialize(&value)
                        .map_err(|e| ZlfError::Serialization(e.to_string()))?;
                    
                    *scores.entry(entry.node_id).or_insert(0.0) += entry.score;
                }
            }
        }
        
        let mut results: Vec<(String, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(results)
    }

    pub fn remove_entry(&self, node_id: &str, token: &str) -> Result<()> {
        let key = format!("bm25:{}:{}", token, node_id);
        
        self.db.delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    pub fn remove_all_for_node(&self, node_id: &str) -> Result<()> {
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        
        for item in iter {
            let (key, _) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);
            
            if key_str.contains(&format!(":{}", node_id)) {
                self.db.delete(&key)
                    .map_err(|e| ZlfError::Internal(e.to_string()))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_index() -> (BM25Index, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let index = BM25Index::open(temp_dir.path().join("bm25")).unwrap();
        (index, temp_dir)
    }

    #[test]
    fn test_add_and_search() {
        let (index, _temp) = create_test_index();
        
        index.add_entry(BM25Entry {
            node_id: "alice".to_string(),
            token: "engineer".to_string(),
            score: 2.5,
        }).unwrap();
        
        index.add_entry(BM25Entry {
            node_id: "bob".to_string(),
            token: "software".to_string(),
            score: 1.5,
        }).unwrap();
        
        let results = index.search("engineer").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "alice");
        assert_eq!(results[0].1, 2.5);
    }

    #[test]
    fn test_search_multiple_tokens() {
        let (index, _temp) = create_test_index();
        
        index.add_entry(BM25Entry {
            node_id: "alice".to_string(),
            token: "software".to_string(),
            score: 1.0,
        }).unwrap();
        
        index.add_entry(BM25Entry {
            node_id: "alice".to_string(),
            token: "engineer".to_string(),
            score: 1.5,
        }).unwrap();
        
        index.add_entry(BM25Entry {
            node_id: "bob".to_string(),
            token: "software".to_string(),
            score: 1.0,
        }).unwrap();
        
        let results = index.search("software engineer").unwrap();
        assert_eq!(results.len(), 2);
        // alice should have higher score (1.0 + 1.5 = 2.5)
        assert_eq!(results[0].0, "alice");
    }

    #[test]
    fn test_remove_entry() {
        let (index, _temp) = create_test_index();
        
        index.add_entry(BM25Entry {
            node_id: "alice".to_string(),
            token: "engineer".to_string(),
            score: 2.5,
        }).unwrap();
        
        index.remove_entry("alice", "engineer").unwrap();
        
        let results = index.search("engineer").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_empty_query() {
        let (index, _temp) = create_test_index();
        
        index.add_entry(BM25Entry {
            node_id: "alice".to_string(),
            token: "engineer".to_string(),
            score: 2.5,
        }).unwrap();
        
        let results = index.search("").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_chinese_tokenization() {
        let (index, _temp) = create_test_index();
        
        // Test Chinese tokenization
        let tokens = index.tokenize("我们中出了一个叛徒");
        assert!(tokens.contains(&"我们".to_string()));
        assert!(tokens.contains(&"叛徒".to_string()));
    }

    #[test]
    fn test_index_text_with_chinese() {
        let (index, _temp) = create_test_index();
        
        index.index_text("alice", "软件工程师").unwrap();
        
        let results = index.search("软件").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "alice");
    }

    #[test]
    fn test_mixed_chinese_and_english() {
        let (index, _temp) = create_test_index();
        
        index.index_text("alice", "Alice is a 软件工程师").unwrap();
        
        // Search in English
        let results = index.search("Alice").unwrap();
        assert_eq!(results.len(), 1);
        
        // Search in Chinese
        let results = index.search("软件").unwrap();
        assert_eq!(results.len(), 1);
    }
}
