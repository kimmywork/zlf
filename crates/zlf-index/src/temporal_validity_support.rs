use std::cmp::Ordering;
use std::collections::BinaryHeap;

use serde::{Deserialize, Serialize};
use zlf_core::{Result, ZlfError};

use crate::temporal_validity::{
    END_PREFIX, ENTITY_PREFIX, OPEN_PREFIX, START_PREFIX, STATS_PREFIX,
};
use crate::{
    encode_ordered_micros, GenerationId, IndexDocumentId, TemporalAccessPath, ValidityQueryResult,
    ValidityRecord,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ValidityStats {
    total: u64,
    open: u64,
    min_start: Option<i64>,
    max_start: Option<i64>,
    min_end: Option<i64>,
    max_end: Option<i64>,
}

impl ValidityStats {
    pub(crate) fn observe(&mut self, record: &ValidityRecord) {
        self.total += 1;
        update_bounds(
            &mut self.min_start,
            &mut self.max_start,
            record.valid_from_micros,
        );
        if let Some(end) = record.valid_to_micros {
            update_bounds(&mut self.min_end, &mut self.max_end, end);
        } else {
            self.open += 1;
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct HeapValidity(pub(crate) ValidityRecord);

impl Ord for HeapValidity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .valid_from_micros
            .cmp(&other.0.valid_from_micros)
            .then_with(|| self.0.id.cmp(&other.0.id))
    }
}

impl PartialOrd for HeapValidity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(crate) fn result(
    heap: BinaryHeap<HeapValidity>,
    candidates: u64,
    access_path: TemporalAccessPath,
) -> ValidityQueryResult {
    let mut records = heap.into_iter().map(|item| item.0).collect::<Vec<_>>();
    records.sort_by(|left, right| {
        left.valid_from_micros
            .cmp(&right.valid_from_micros)
            .then_with(|| left.id.cmp(&right.id))
    });
    ValidityQueryResult {
        records,
        candidates_scanned: candidates,
        access_path,
    }
}

pub(crate) fn prefer_start(stats: &ValidityStats, _query_start: i64, query_end: i64) -> bool {
    let start_estimate = estimate_le(stats.total, stats.min_start, stats.max_start, query_end);
    let finite = stats.total.saturating_sub(stats.open);
    let end_estimate = finite.saturating_sub(estimate_le(
        finite,
        stats.min_end,
        stats.max_end,
        _query_start,
    )) + stats.open;
    start_estimate <= end_estimate
}

fn estimate_le(total: u64, min: Option<i64>, max: Option<i64>, value: i64) -> u64 {
    let (Some(min), Some(max)) = (min, max) else {
        return 0;
    };
    if value < min {
        0
    } else if value >= max || min == max {
        total
    } else {
        let fraction = (value - min) as f64 / (max - min) as f64;
        (fraction * total as f64).round() as u64
    }
}

fn update_bounds(min: &mut Option<i64>, max: &mut Option<i64>, value: i64) {
    *min = Some(min.map_or(value, |current| current.min(value)));
    *max = Some(max.map_or(value, |current| current.max(value)));
}

pub(crate) fn start_key(record: &ValidityRecord) -> Vec<u8> {
    time_record_key(
        START_PREFIX,
        &record.generation,
        record.valid_from_micros,
        &record.id.0,
    )
}

pub(crate) fn end_key(record: &ValidityRecord) -> Vec<u8> {
    time_record_key(
        END_PREFIX,
        &record.generation,
        record.valid_to_micros.expect("finite end"),
        &record.id.0,
    )
}

pub(crate) fn open_key(record: &ValidityRecord) -> Vec<u8> {
    let mut key = generation_prefix(OPEN_PREFIX, &record.generation);
    push_part(&mut key, record.id.0.as_bytes());
    key
}

pub(crate) fn entity_key(record: &ValidityRecord) -> Vec<u8> {
    let mut key = entity_document_prefix(&record.generation, &record.document_id);
    push_part(&mut key, record.id.0.as_bytes());
    key
}

pub(crate) fn entity_document_prefix(
    generation: &GenerationId,
    document: &IndexDocumentId,
) -> Vec<u8> {
    let mut key = generation_prefix(ENTITY_PREFIX, generation);
    push_part(&mut key, &document.canonical_bytes());
    key
}

pub(crate) fn time_record_key(
    prefix: &[u8],
    generation: &GenerationId,
    time: i64,
    id: &str,
) -> Vec<u8> {
    let base = generation_prefix(prefix, generation);
    let mut key = time_seek_key(&base, time);
    push_part(&mut key, id.as_bytes());
    key
}

pub(crate) fn time_seek_key(prefix: &[u8], time: i64) -> Vec<u8> {
    let mut key = prefix.to_vec();
    key.extend_from_slice(&encode_ordered_micros(time));
    key
}

pub(crate) fn upper_seek(prefix: &[u8]) -> Vec<u8> {
    let mut key = prefix.to_vec();
    key.push(0xff);
    key
}

pub(crate) fn generation_prefix(prefix: &[u8], generation: &GenerationId) -> Vec<u8> {
    let mut key = prefix.to_vec();
    push_part(&mut key, generation.0.as_bytes());
    key
}

pub(crate) fn stats_key(generation: &GenerationId) -> Vec<u8> {
    generation_prefix(STATS_PREFIX, generation)
}

fn push_part(target: &mut Vec<u8>, part: &[u8]) {
    target.extend_from_slice(&(part.len() as u32).to_be_bytes());
    target.extend_from_slice(part);
}

pub(crate) fn validate_limit(generation: &GenerationId, limit: usize) -> Result<()> {
    if generation.0.is_empty() || limit == 0 {
        return Err(ZlfError::Internal(
            "validity query requires generation and positive limit".into(),
        ));
    }
    Ok(())
}

pub(crate) fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}

pub(crate) fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
