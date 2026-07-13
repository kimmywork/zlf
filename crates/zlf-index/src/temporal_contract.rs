use chrono::{DateTime, Days, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::{GenerationId, IndexDocumentId};

pub const TEMPORAL_RECORD_SCHEMA_VERSION: u32 = 1;
pub const TEMPORAL_PARSER_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TemporalRecordId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRecord {
    pub schema_version: u32,
    pub generation: GenerationId,
    pub id: TemporalRecordId,
    pub document_id: IndexDocumentId,
    pub source_version: u64,
    pub at_micros: i64,
}

impl EventRecord {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != TEMPORAL_RECORD_SCHEMA_VERSION
            || self.generation.0.is_empty()
            || self.id.0.is_empty()
        {
            return Err("invalid event record".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidityRecord {
    pub schema_version: u32,
    pub generation: GenerationId,
    pub id: TemporalRecordId,
    pub document_id: IndexDocumentId,
    pub source_version: u64,
    pub valid_from_micros: i64,
    pub valid_to_micros: Option<i64>,
}

impl ValidityRecord {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != TEMPORAL_RECORD_SCHEMA_VERSION
            || self.generation.0.is_empty()
            || self.id.0.is_empty()
            || self
                .valid_to_micros
                .is_some_and(|end| self.valid_from_micros >= end)
        {
            return Err("invalid half-open validity record".into());
        }
        Ok(())
    }

    pub fn contains(&self, instant: i64) -> bool {
        self.valid_from_micros <= instant && self.valid_to_micros.is_none_or(|end| instant < end)
    }

    pub fn overlaps(&self, start: i64, end: i64) -> bool {
        self.valid_from_micros < end
            && self
                .valid_to_micros
                .is_none_or(|record_end| record_end > start)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalAccessPath {
    EventByTime,
    ValidByStart,
    ValidByEnd,
    ValidOpenEnd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalRecord {
    Event(EventRecord),
    Validity(ValidityRecord),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalHit {
    pub record: TemporalRecord,
    pub generation: GenerationId,
    pub watermark: u64,
    pub access_path: TemporalAccessPath,
    pub candidates_scanned: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQueryResult {
    pub records: Vec<EventRecord>,
    pub candidates_scanned: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidityQueryResult {
    pub records: Vec<ValidityRecord>,
    pub candidates_scanned: u64,
    pub access_path: TemporalAccessPath,
}

pub fn parse_utc_micros(input: &str) -> Result<i64, String> {
    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return utc_midnight(date);
    }
    let instant = DateTime::parse_from_rfc3339(input).map_err(|_| {
        "instant must be RFC3339 with an explicit offset or an ISO date".to_string()
    })?;
    Ok(instant.with_timezone(&Utc).timestamp_micros())
}

pub fn utc_day_range(input: &str) -> Result<(i64, i64), String> {
    let date = NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .map_err(|_| "day must use YYYY-MM-DD".to_string())?;
    let next = date
        .checked_add_days(Days::new(1))
        .ok_or_else(|| "day boundary overflow".to_string())?;
    Ok((utc_midnight(date)?, utc_midnight(next)?))
}

pub fn validate_half_open_range(start: i64, end: i64) -> Result<(), String> {
    if start >= end {
        return Err("half-open range requires start < end".into());
    }
    Ok(())
}

pub fn encode_ordered_micros(value: i64) -> [u8; 8] {
    ((value as u64) ^ (1_u64 << 63)).to_be_bytes()
}

pub fn decode_ordered_micros(bytes: [u8; 8]) -> i64 {
    (u64::from_be_bytes(bytes) ^ (1_u64 << 63)) as i64
}

pub fn event_range_oracle(records: &[EventRecord], start: i64, end: i64) -> Vec<EventRecord> {
    let mut matches = records
        .iter()
        .filter(|record| start <= record.at_micros && record.at_micros < end)
        .cloned()
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        left.at_micros
            .cmp(&right.at_micros)
            .then_with(|| left.id.cmp(&right.id))
    });
    matches
}

pub fn valid_at_oracle(records: &[ValidityRecord], instant: i64) -> Vec<ValidityRecord> {
    sorted_valid(records.iter().filter(|record| record.contains(instant)))
}

pub fn valid_overlaps_oracle(
    records: &[ValidityRecord],
    start: i64,
    end: i64,
) -> Vec<ValidityRecord> {
    sorted_valid(records.iter().filter(|record| record.overlaps(start, end)))
}

fn sorted_valid<'a>(records: impl Iterator<Item = &'a ValidityRecord>) -> Vec<ValidityRecord> {
    let mut matches = records.cloned().collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        left.valid_from_micros
            .cmp(&right.valid_from_micros)
            .then_with(|| left.id.cmp(&right.id))
    });
    matches
}

fn utc_midnight(date: NaiveDate) -> Result<i64, String> {
    let naive = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| "invalid UTC day boundary".to_string())?;
    Ok(Utc.from_utc_datetime(&naive).timestamp_micros())
}
