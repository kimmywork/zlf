use zlf_core::{Result, ZlfError};

use crate::Storage;

impl Storage {
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
