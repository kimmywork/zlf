use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};

use zlf_core::{Result, ZlfError};

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

        Ok(Self { db: Arc::new(db) })
    }

    pub fn add_entry(&self, entry: TemporalEntry) -> Result<()> {
        // Index by valid_from date
        let date_key = entry.valid_from.format("%Y-%m-%d").to_string();
        let key = format!("temporal:{}:{}", date_key, entry.node_id);

        let data =
            bincode::serialize(&entry).map_err(|e| ZlfError::Serialization(e.to_string()))?;

        self.db
            .put(&key, data)
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

    pub fn get_entries_in_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<TemporalEntry>> {
        let mut entries = Vec::new();

        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (_, value) = item.map_err(|e| ZlfError::Internal(e.to_string()))?;

            let entry: TemporalEntry =
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))?;

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

        self.db
            .delete(&key)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;

        Ok(())
    }

    pub fn time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<TemporalEntry>> {
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

            let entry: TemporalEntry =
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))?;

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

            let entry: TemporalEntry =
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))?;

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
