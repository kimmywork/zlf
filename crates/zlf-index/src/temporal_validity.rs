use std::collections::{BTreeSet, BinaryHeap};
use std::path::Path;
use std::sync::Arc;

use rocksdb::{Direction, IteratorMode, Options, WriteBatch, DB};
use zlf_core::{Result, ZlfError};

use crate::temporal_validity_support::{
    end_key, entity_document_prefix, entity_key, generation_prefix, internal, open_key,
    prefer_start, result, serialization, start_key, stats_key, time_seek_key, upper_seek,
    validate_limit, HeapValidity, ValidityStats,
};
use crate::{
    validate_half_open_range, GenerationId, IndexDocumentId, TemporalAccessPath,
    ValidityQueryResult, ValidityRecord,
};

pub(crate) const START_PREFIX: &[u8] = b"temporal:v1:valid:start:";
pub(crate) const END_PREFIX: &[u8] = b"temporal:v1:valid:end:";
pub(crate) const OPEN_PREFIX: &[u8] = b"temporal:v1:valid:open:";
pub(crate) const ENTITY_PREFIX: &[u8] = b"temporal:v1:valid:entity:";
pub(crate) const STATS_PREFIX: &[u8] = b"temporal:v1:valid:stats:";
const SCHEMA_KEY: &[u8] = b"temporal:v1:valid:schema";

#[derive(Clone)]
pub struct ValidityStore {
    db: Arc<DB>,
}

impl ValidityStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut options = Options::default();
        options.create_if_missing(true);
        let db = DB::open(&options, path).map_err(internal)?;
        match db.get(SCHEMA_KEY).map_err(internal)? {
            Some(value) if value.as_slice() != b"1" => {
                return Err(ZlfError::Internal("incompatible validity schema".into()))
            }
            None => db.put(SCHEMA_KEY, b"1").map_err(internal)?,
            _ => {}
        }
        Ok(Self { db: Arc::new(db) })
    }

    pub fn put(&self, record: &ValidityRecord) -> Result<()> {
        self.apply(std::slice::from_ref(record), &[])
    }

    pub fn delete(&self, record: &ValidityRecord) -> Result<()> {
        self.apply(&[], std::slice::from_ref(record))
    }

    pub fn apply(&self, upserts: &[ValidityRecord], deletes: &[ValidityRecord]) -> Result<()> {
        let mut batch = WriteBatch::default();
        let mut generations = BTreeSet::new();
        for record in deletes {
            generations.insert(record.generation.clone());
            batch.delete(start_key(record));
            match record.valid_to_micros {
                Some(_) => batch.delete(end_key(record)),
                None => batch.delete(open_key(record)),
            }
            batch.delete(entity_key(record));
        }
        for record in upserts {
            record.validate().map_err(ZlfError::Internal)?;
            generations.insert(record.generation.clone());
            let value = bincode::serialize(record).map_err(serialization)?;
            batch.put(start_key(record), &value);
            match record.valid_to_micros {
                Some(_) => batch.put(end_key(record), &value),
                None => batch.put(open_key(record), &value),
            }
            batch.put(entity_key(record), &value);
        }
        self.db.write(batch).map_err(internal)?;
        for generation in generations {
            self.refresh_stats(&generation)?;
        }
        Ok(())
    }

    pub fn valid_at(
        &self,
        generation: &GenerationId,
        instant: i64,
        limit: usize,
    ) -> Result<ValidityQueryResult> {
        validate_limit(generation, limit)?;
        let stats = self.stats(generation)?;
        if prefer_start(&stats, instant, instant.saturating_add(1)) {
            self.scan_start(generation, instant.checked_add(1), limit, |record| {
                record.contains(instant)
            })
        } else {
            self.scan_end(
                generation,
                instant,
                instant.saturating_add(1),
                limit,
                |record| record.contains(instant),
            )
        }
    }

    pub fn overlaps(
        &self,
        generation: &GenerationId,
        start: i64,
        end: i64,
        limit: usize,
    ) -> Result<ValidityQueryResult> {
        validate_limit(generation, limit)?;
        validate_half_open_range(start, end).map_err(ZlfError::Internal)?;
        let stats = self.stats(generation)?;
        if prefer_start(&stats, start, end) {
            self.scan_start(generation, Some(end), limit, |record| {
                record.overlaps(start, end)
            })
        } else {
            self.scan_end(generation, start, end, limit, |record| {
                record.overlaps(start, end)
            })
        }
    }

    pub fn for_document(
        &self,
        generation: &GenerationId,
        document_id: &IndexDocumentId,
        limit: usize,
    ) -> Result<ValidityQueryResult> {
        validate_limit(generation, limit)?;
        let prefix = entity_document_prefix(generation, document_id);
        self.scan_records(
            &prefix,
            |key| key.starts_with(&prefix),
            limit,
            |_| true,
            TemporalAccessPath::ValidByStart,
        )
    }

    fn scan_start(
        &self,
        generation: &GenerationId,
        exclusive_end: Option<i64>,
        limit: usize,
        matches: impl Fn(&ValidityRecord) -> bool,
    ) -> Result<ValidityQueryResult> {
        let prefix = generation_prefix(START_PREFIX, generation);
        let end_key = exclusive_end.map(|end| time_seek_key(&prefix, end));
        self.scan_records(
            &prefix,
            |key| {
                end_key
                    .as_ref()
                    .map_or_else(|| key.starts_with(&prefix), |end| key < end.as_slice())
            },
            limit,
            matches,
            TemporalAccessPath::ValidByStart,
        )
    }

    fn scan_end(
        &self,
        generation: &GenerationId,
        exclusive_start: i64,
        _query_end: i64,
        limit: usize,
        matches: impl Fn(&ValidityRecord) -> bool,
    ) -> Result<ValidityQueryResult> {
        let end_prefix = generation_prefix(END_PREFIX, generation);
        let start = exclusive_start.checked_add(1).map_or_else(
            || upper_seek(&end_prefix),
            |value| time_seek_key(&end_prefix, value),
        );
        let mut heap = BinaryHeap::new();
        let mut candidates = self.collect(
            &start,
            |key| key.starts_with(&end_prefix),
            limit,
            &matches,
            &mut heap,
        )?;
        let open_prefix = generation_prefix(OPEN_PREFIX, generation);
        candidates += self.collect(
            &open_prefix,
            |key| key.starts_with(&open_prefix),
            limit,
            &matches,
            &mut heap,
        )?;
        Ok(result(heap, candidates, TemporalAccessPath::ValidByEnd))
    }

    fn scan_records(
        &self,
        start: &[u8],
        include: impl Fn(&[u8]) -> bool,
        limit: usize,
        matches: impl Fn(&ValidityRecord) -> bool,
        access_path: TemporalAccessPath,
    ) -> Result<ValidityQueryResult> {
        let mut heap = BinaryHeap::new();
        let candidates = self.collect(start, include, limit, &matches, &mut heap)?;
        Ok(result(heap, candidates, access_path))
    }

    fn collect(
        &self,
        start: &[u8],
        include: impl Fn(&[u8]) -> bool,
        limit: usize,
        matches: &impl Fn(&ValidityRecord) -> bool,
        heap: &mut BinaryHeap<HeapValidity>,
    ) -> Result<u64> {
        let mut candidates = 0;
        for item in self
            .db
            .iterator(IteratorMode::From(start, Direction::Forward))
        {
            let (key, value) = item.map_err(internal)?;
            if !include(&key) {
                break;
            }
            candidates += 1;
            let record: ValidityRecord = bincode::deserialize(&value).map_err(serialization)?;
            if matches(&record) {
                heap.push(HeapValidity(record));
                if heap.len() > limit {
                    heap.pop();
                }
            }
        }
        Ok(candidates)
    }

    fn refresh_stats(&self, generation: &GenerationId) -> Result<()> {
        let prefix = generation_prefix(START_PREFIX, generation);
        let mut stats = ValidityStats::default();
        for item in self
            .db
            .iterator(IteratorMode::From(&prefix, Direction::Forward))
        {
            let (key, value) = item.map_err(internal)?;
            if !key.starts_with(&prefix) {
                break;
            }
            let record: ValidityRecord = bincode::deserialize(&value).map_err(serialization)?;
            stats.observe(&record);
        }
        self.db
            .put(
                stats_key(generation),
                bincode::serialize(&stats).map_err(serialization)?,
            )
            .map_err(internal)
    }

    fn stats(&self, generation: &GenerationId) -> Result<ValidityStats> {
        self.db
            .get(stats_key(generation))
            .map_err(internal)?
            .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
            .transpose()
            .map(|stats| stats.unwrap_or_default())
    }
}
