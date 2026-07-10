use rocksdb::{Direction, IteratorMode, WriteBatch};
use zlf_core::{Result, ZlfError};

use crate::Storage;

#[derive(Debug, Clone)]
pub enum RawMutation {
    Put(Vec<u8>, Vec<u8>),
    Delete(Vec<u8>),
}

impl Storage {
    pub fn get_raw(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.db
            .get(key)
            .map(|value| value.map(|bytes| bytes.to_vec()))
            .map_err(|error| ZlfError::Internal(error.to_string()))
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

    pub fn write_raw_batch(&self, mutations: &[RawMutation]) -> Result<()> {
        let mut batch = WriteBatch::default();
        for mutation in mutations {
            match mutation {
                RawMutation::Put(key, value) => batch.put(key, value),
                RawMutation::Delete(key) => batch.delete(key),
            }
        }
        self.db
            .write(batch)
            .map_err(|error| ZlfError::Internal(error.to_string()))
    }

    pub fn scan_prefix(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let mut results = Vec::new();
        let iter = self
            .db
            .iterator(IteratorMode::From(prefix.as_bytes(), Direction::Forward));
        for item in iter {
            let (key, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            if !key.starts_with(prefix.as_bytes()) {
                break;
            }
            results.push((String::from_utf8_lossy(&key).to_string(), value.to_vec()));
        }
        Ok(results)
    }

    pub fn close(&self) {
        // DB is closed when dropped
    }
}
