use std::path::Path;
use std::sync::Arc;

use rocksdb::{Direction, IteratorMode, Options, WriteBatch, DB};
use zlf_core::{Result, ZlfError};

use crate::{
    encode_ordered_micros, utc_day_range, validate_half_open_range, EventQueryResult, EventRecord,
    GenerationId, IndexDocumentId,
};

const TIME_PREFIX: &[u8] = b"temporal:v1:event:time:";
const ENTITY_PREFIX: &[u8] = b"temporal:v1:event:entity:";
const SCHEMA_KEY: &[u8] = b"temporal:v1:event:schema";
const SCHEMA_VALUE: &[u8] = b"1";

#[derive(Clone)]
pub struct EventTimeStore {
    db: Arc<DB>,
}

impl EventTimeStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut options = Options::default();
        options.create_if_missing(true);
        let db = DB::open(&options, path).map_err(internal)?;
        match db.get(SCHEMA_KEY).map_err(internal)? {
            Some(value) if value.as_slice() != SCHEMA_VALUE => {
                return Err(ZlfError::Internal("incompatible event-time schema".into()))
            }
            None => db.put(SCHEMA_KEY, SCHEMA_VALUE).map_err(internal)?,
            _ => {}
        }
        Ok(Self { db: Arc::new(db) })
    }

    pub fn put(&self, record: &EventRecord) -> Result<()> {
        record.validate().map_err(ZlfError::Internal)?;
        let value = bincode::serialize(record).map_err(serialization)?;
        let mut batch = WriteBatch::default();
        batch.put(time_key(record), &value);
        batch.put(entity_key(record), &value);
        self.db.write(batch).map_err(internal)
    }

    pub fn delete(&self, record: &EventRecord) -> Result<()> {
        let mut batch = WriteBatch::default();
        batch.delete(time_key(record));
        batch.delete(entity_key(record));
        self.db.write(batch).map_err(internal)
    }

    pub fn day(
        &self,
        generation: &GenerationId,
        date: &str,
        limit: usize,
    ) -> Result<EventQueryResult> {
        let (start, end) = utc_day_range(date).map_err(ZlfError::Internal)?;
        self.range(generation, start, end, limit)
    }

    pub fn range(
        &self,
        generation: &GenerationId,
        start: i64,
        end: i64,
        limit: usize,
    ) -> Result<EventQueryResult> {
        validate_query(generation, start, end, limit)?;
        let prefix = time_generation_prefix(generation);
        let start_key = time_seek_key(&prefix, start);
        let end_key = time_seek_key(&prefix, end);
        self.scan_forward(&start_key, |key| key < end_key.as_slice(), limit)
    }

    pub fn after(
        &self,
        generation: &GenerationId,
        instant: i64,
        limit: usize,
    ) -> Result<EventQueryResult> {
        validate_limit(generation, limit)?;
        let prefix = time_generation_prefix(generation);
        let Some(next) = instant.checked_add(1) else {
            return Ok(EventQueryResult {
                records: Vec::new(),
                candidates_scanned: 0,
            });
        };
        let start_key = time_seek_key(&prefix, next);
        self.scan_forward(&start_key, |key| key.starts_with(&prefix), limit)
    }

    pub fn before(
        &self,
        generation: &GenerationId,
        instant: i64,
        limit: usize,
    ) -> Result<EventQueryResult> {
        validate_limit(generation, limit)?;
        let prefix = time_generation_prefix(generation);
        let start_key = time_seek_key(&prefix, instant);
        let mut records = Vec::new();
        let mut candidates = 0;
        for item in self
            .db
            .iterator(IteratorMode::From(&start_key, Direction::Reverse))
        {
            let (key, value) = item.map_err(internal)?;
            if !key.starts_with(&prefix) {
                break;
            }
            candidates += 1;
            records.push(deserialize(&value)?);
            if records.len() == limit {
                break;
            }
        }
        records.reverse();
        Ok(EventQueryResult {
            records,
            candidates_scanned: candidates,
        })
    }

    pub fn for_document(
        &self,
        generation: &GenerationId,
        document_id: &IndexDocumentId,
        limit: usize,
    ) -> Result<EventQueryResult> {
        validate_limit(generation, limit)?;
        let prefix = entity_document_prefix(generation, document_id);
        self.scan_forward(&prefix, |key| key.starts_with(&prefix), limit)
    }

    fn scan_forward(
        &self,
        start_key: &[u8],
        include: impl Fn(&[u8]) -> bool,
        limit: usize,
    ) -> Result<EventQueryResult> {
        let mut records = Vec::new();
        let mut candidates = 0;
        for item in self
            .db
            .iterator(IteratorMode::From(start_key, Direction::Forward))
        {
            let (key, value) = item.map_err(internal)?;
            if !include(&key) {
                break;
            }
            candidates += 1;
            records.push(deserialize(&value)?);
            if records.len() == limit {
                break;
            }
        }
        Ok(EventQueryResult {
            records,
            candidates_scanned: candidates,
        })
    }
}

fn validate_query(generation: &GenerationId, start: i64, end: i64, limit: usize) -> Result<()> {
    validate_limit(generation, limit)?;
    validate_half_open_range(start, end).map_err(ZlfError::Internal)
}

fn validate_limit(generation: &GenerationId, limit: usize) -> Result<()> {
    if generation.0.is_empty() || limit == 0 {
        return Err(ZlfError::Internal(
            "event query requires generation and positive limit".into(),
        ));
    }
    Ok(())
}

fn time_key(record: &EventRecord) -> Vec<u8> {
    let prefix = time_generation_prefix(&record.generation);
    let mut key = time_seek_key(&prefix, record.at_micros);
    push_part(&mut key, record.id.0.as_bytes());
    key
}

fn time_generation_prefix(generation: &GenerationId) -> Vec<u8> {
    let mut key = TIME_PREFIX.to_vec();
    push_part(&mut key, generation.0.as_bytes());
    key
}

fn time_seek_key(prefix: &[u8], instant: i64) -> Vec<u8> {
    let mut key = prefix.to_vec();
    key.extend_from_slice(&encode_ordered_micros(instant));
    key
}

fn entity_key(record: &EventRecord) -> Vec<u8> {
    let mut key = entity_document_prefix(&record.generation, &record.document_id);
    push_part(&mut key, record.id.0.as_bytes());
    key
}

fn entity_document_prefix(generation: &GenerationId, document_id: &IndexDocumentId) -> Vec<u8> {
    let mut key = ENTITY_PREFIX.to_vec();
    push_part(&mut key, generation.0.as_bytes());
    push_part(&mut key, &document_id.canonical_bytes());
    key
}

fn push_part(target: &mut Vec<u8>, part: &[u8]) {
    target.extend_from_slice(&(part.len() as u32).to_be_bytes());
    target.extend_from_slice(part);
}

fn deserialize(bytes: &[u8]) -> Result<EventRecord> {
    bincode::deserialize(bytes).map_err(serialization)
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
