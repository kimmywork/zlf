use std::path::Path;
use std::sync::Arc;

use rocksdb::{Options, DB};
use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};

use zlf_core::{ZlfError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEntry {
    pub node_id: String,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}

pub struct TemporalIndex {
    db: Arc<DB>,
}

impl TemporalIndex {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        let db = DB::open(&opts, path)
            .map_err(|e| ZlfError::Internal(format!("Failed to open temporal index: {}", e)))?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    pub fn add_entry(&self, entry: TemporalEntry) -> Result<()> {
        // Index by valid_from date
        let date_key = entry.valid_from.format("%Y-%m-%d").to_string();
        let key = format!("temporal:{}:{}", date_key, entry.node_id);
        
        let data = bincode::serialize(&entry)
            .map_err(|e| ZlfError::Serialization(e.to_string()))?;
        
        self.db.put(&key, data)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    pub fn get_entries_for_date(&self, date: NaiveDate) -> Result<Vec<TemporalEntry>> {
        let prefix = format!("temporal:{}:", date.format("%Y-%m-%d"));
        let mut entries = Vec::new();
        
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            let key_str = String::from_utf8_lossy(&key);
            
            if key_str.starts_with(&prefix) {
                let entry: TemporalEntry = bincode::deserialize(&value)
                    .map_err(|e| ZlfError::Serialization(e.to_string()))?;
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    pub fn get_entries_in_range(&self, start: NaiveDate, end: NaiveDate) -> Result<Vec<TemporalEntry>> {
        let mut entries = Vec::new();
        
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (_, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            
            let entry: TemporalEntry = bincode::deserialize(&value)
                .map_err(|e| ZlfError::Serialization(e.to_string()))?;
            
            let entry_date = entry.valid_from.date_naive();
            if entry_date >= start && entry_date <= end {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    pub fn remove_entry(&self, node_id: &str, valid_from: DateTime<Utc>) -> Result<()> {
        let date_key = valid_from.format("%Y-%m-%d").to_string();
        let key = format!("temporal:{}:{}", date_key, node_id);
        
        self.db.delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    pub fn time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<TemporalEntry>> {
        let start_date = start.date_naive();
        let end_date = end.date_naive();
        self.get_entries_in_range(start_date, end_date)
    }

    pub fn before(&self, timestamp: DateTime<Utc>) -> Result<Vec<TemporalEntry>> {
        let mut entries = Vec::new();
        let target_date = timestamp.date_naive();
        
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (_, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            
            let entry: TemporalEntry = bincode::deserialize(&value)
                .map_err(|e| ZlfError::Serialization(e.to_string()))?;
            
            if entry.valid_from.date_naive() <= target_date {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    pub fn after(&self, timestamp: DateTime<Utc>) -> Result<Vec<TemporalEntry>> {
        let mut entries = Vec::new();
        let target_date = timestamp.date_naive();
        
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (_, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;
            
            let entry: TemporalEntry = bincode::deserialize(&value)
                .map_err(|e| ZlfError::Serialization(e.to_string()))?;
            
            if entry.valid_from.date_naive() >= target_date {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    pub fn between(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<TemporalEntry>> {
        self.time_range(start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_index() -> (TemporalIndex, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let index = TemporalIndex::open(temp_dir.path().join("temporal")).unwrap();
        (index, temp_dir)
    }

    #[test]
    fn test_add_and_get_entry() {
        let (index, _temp) = create_test_index();
        
        let entry = TemporalEntry {
            node_id: "alice".to_string(),
            valid_from: Utc::now(),
            valid_to: None,
        };
        
        index.add_entry(entry.clone()).unwrap();
        
        let date = entry.valid_from.date_naive();
        let entries = index.get_entries_for_date(date).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].node_id, "alice");
    }

    #[test]
    fn test_get_entries_in_range() {
        let (index, _temp) = create_test_index();
        
        let entry1 = TemporalEntry {
            node_id: "alice".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        let entry2 = TemporalEntry {
            node_id: "bob".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-06-15T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        index.add_entry(entry1).unwrap();
        index.add_entry(entry2).unwrap();
        
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 6, 30).unwrap();
        
        let entries = index.get_entries_in_range(start, end).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_remove_entry() {
        let (index, _temp) = create_test_index();
        
        let entry = TemporalEntry {
            node_id: "alice".to_string(),
            valid_from: Utc::now(),
            valid_to: None,
        };
        
        index.add_entry(entry.clone()).unwrap();
        
        index.remove_entry("alice", entry.valid_from).unwrap();
        
        let date = entry.valid_from.date_naive();
        let entries = index.get_entries_for_date(date).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_time_range() {
        let (index, _temp) = create_test_index();
        
        let entry1 = TemporalEntry {
            node_id: "alice".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        let entry2 = TemporalEntry {
            node_id: "bob".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-06-15T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        index.add_entry(entry1).unwrap();
        index.add_entry(entry2).unwrap();
        
        let start = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z").unwrap().with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2026-06-30T00:00:00Z").unwrap().with_timezone(&Utc);
        
        let entries = index.time_range(start, end).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_before() {
        let (index, _temp) = create_test_index();
        
        let entry1 = TemporalEntry {
            node_id: "alice".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        let entry2 = TemporalEntry {
            node_id: "bob".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-06-15T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        index.add_entry(entry1).unwrap();
        index.add_entry(entry2).unwrap();
        
        let before_date = DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z").unwrap().with_timezone(&Utc);
        
        let entries = index.before(before_date).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].node_id, "alice");
    }

    #[test]
    fn test_after() {
        let (index, _temp) = create_test_index();
        
        let entry1 = TemporalEntry {
            node_id: "alice".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        let entry2 = TemporalEntry {
            node_id: "bob".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-06-15T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        index.add_entry(entry1).unwrap();
        index.add_entry(entry2).unwrap();
        
        let after_date = DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z").unwrap().with_timezone(&Utc);
        
        let entries = index.after(after_date).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].node_id, "bob");
    }

    #[test]
    fn test_between() {
        let (index, _temp) = create_test_index();
        
        let entry1 = TemporalEntry {
            node_id: "alice".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        let entry2 = TemporalEntry {
            node_id: "bob".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-06-15T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        let entry3 = TemporalEntry {
            node_id: "charlie".to_string(),
            valid_from: DateTime::parse_from_rfc3339("2026-12-01T00:00:00Z").unwrap().with_timezone(&Utc),
            valid_to: None,
        };
        
        index.add_entry(entry1).unwrap();
        index.add_entry(entry2).unwrap();
        index.add_entry(entry3).unwrap();
        
        let start = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z").unwrap().with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2026-06-30T00:00:00Z").unwrap().with_timezone(&Utc);
        
        let entries = index.between(start, end).unwrap();
        assert_eq!(entries.len(), 2);
    }
}
