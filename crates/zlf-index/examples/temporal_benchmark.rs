use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;
use zlf_core::EntityRef;
use zlf_index::{
    event_range_oracle, valid_at_oracle, valid_overlaps_oracle, EventRecord, EventTimeStore,
    GenerationId, IndexDocumentId, TemporalRecordId, ValidityRecord, ValidityStore,
    TEMPORAL_RECORD_SCHEMA_VERSION,
};

const MINUTE: i64 = 60_000_000;
const DAY: i64 = 1_440 * MINUTE;

#[derive(Clone, Copy)]
enum Distribution {
    Uniform,
    Skewed,
    LongOpen,
}

impl Distribution {
    fn name(self) -> &'static str {
        match self {
            Self::Uniform => "uniform",
            Self::Skewed => "skewed",
            Self::LongOpen => "long_open",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1000".into())
        .parse()?;
    let reports = [
        Distribution::Uniform,
        Distribution::Skewed,
        Distribution::LongOpen,
    ]
    .into_iter()
    .map(|distribution| run_tier(count, distribution))
    .collect::<Result<Vec<_>, _>>()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"zlf-temporal-benchmark-v1",
            "records_per_kind":count,
            "reports":reports,
            "peak_rss_bytes":peak_rss_bytes(),
        }))?
    );
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn run_tier(
    count: usize,
    distribution: Distribution,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let events = EventTimeStore::open(directory.path().join("events"))?;
    let validities = ValidityStore::open(directory.path().join("validities"))?;
    let generation = GenerationId("benchmark-v1".into());
    let event_records = (0..count)
        .map(|index| event(index, event_time(index, distribution), &generation))
        .collect::<Vec<_>>();
    let validity_records = (0..count)
        .map(|index| validity(index, distribution, &generation))
        .collect::<Vec<_>>();
    let started = Instant::now();
    events.apply(&event_records, &[])?;
    validities.apply(&validity_records, &[])?;
    let build = started.elapsed();
    let query_start = query_point(count, distribution);
    let query_end = query_start.saturating_add(DAY);
    let event_expected = event_range_oracle(&event_records, query_start, query_end).len();
    let at_expected = valid_at_oracle(&validity_records, query_start).len();
    let overlap_expected = valid_overlaps_oracle(&validity_records, query_start, query_end).len();
    let (event_latency, event_candidates) = sample(100, || {
        let result = events.range(&generation, query_start, query_end, count.max(1))?;
        assert_eq!(result.records.len(), event_expected);
        Ok::<_, zlf_core::ZlfError>(result.candidates_scanned)
    })?;
    let (at_latency, at_candidates) = sample(100, || {
        let result = validities.valid_at(&generation, query_start, count.max(1))?;
        assert_eq!(result.records.len(), at_expected);
        Ok::<_, zlf_core::ZlfError>(result.candidates_scanned)
    })?;
    let (overlap_latency, overlap_candidates) = sample(100, || {
        let result = validities.overlaps(&generation, query_start, query_end, count.max(1))?;
        assert_eq!(result.records.len(), overlap_expected);
        Ok::<_, zlf_core::ZlfError>(result.candidates_scanned)
    })?;
    let update_count = count.min(100);
    let old_events = &event_records[count - update_count..];
    let old_validities = &validity_records[count - update_count..];
    let new_events = old_events
        .iter()
        .cloned()
        .map(|mut record| {
            record.at_micros = record.at_micros.saturating_add(DAY);
            record.source_version = 2;
            record
        })
        .collect::<Vec<_>>();
    let new_validities = old_validities
        .iter()
        .cloned()
        .map(|mut record| {
            record.valid_from_micros = record.valid_from_micros.saturating_add(DAY);
            record.valid_to_micros = record.valid_to_micros.map(|end| end.saturating_add(DAY));
            record.source_version = 2;
            record
        })
        .collect::<Vec<_>>();
    let update_started = Instant::now();
    events.apply(&new_events, old_events)?;
    validities.apply(&new_validities, old_validities)?;
    let update = update_started.elapsed();
    Ok(json!({
        "distribution":distribution.name(),
        "build_ms":millis(build),
        "build_records_per_second":count.saturating_mul(2) as f64 / build.as_secs_f64(),
        "update_records":update_count * 2,
        "update_ms":millis(update),
        "event_matches":event_expected,
        "valid_at_matches":at_expected,
        "overlap_matches":overlap_expected,
        "event_query_us":latencies(&event_latency),
        "valid_at_query_us":latencies(&at_latency),
        "overlap_query_us":latencies(&overlap_latency),
        "event_candidates":candidate_stats(&event_candidates),
        "valid_at_candidates":candidate_stats(&at_candidates),
        "overlap_candidates":candidate_stats(&overlap_candidates),
        "index_bytes":directory_size(directory.path())?,
    }))
}

fn event_time(index: usize, distribution: Distribution) -> i64 {
    match distribution {
        Distribution::Uniform => index as i64 * MINUTE,
        Distribution::Skewed => (index % 100) as i64 * MINUTE,
        Distribution::LongOpen => index as i64 * 10 * MINUTE,
    }
}

fn validity(index: usize, distribution: Distribution, generation: &GenerationId) -> ValidityRecord {
    let start = event_time(index, distribution);
    let end = match distribution {
        Distribution::Uniform => Some(start + DAY),
        Distribution::Skewed if index.is_multiple_of(3) => None,
        Distribution::Skewed => Some(start + 365 * DAY),
        Distribution::LongOpen if !index.is_multiple_of(5) => None,
        Distribution::LongOpen => Some(start + 1_000 * DAY),
    };
    ValidityRecord {
        schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
        generation: generation.clone(),
        id: TemporalRecordId(format!("valid-{index:010}")),
        document_id: document(index, "valid"),
        source_version: 1,
        valid_from_micros: start,
        valid_to_micros: end,
    }
}

fn event(index: usize, at_micros: i64, generation: &GenerationId) -> EventRecord {
    EventRecord {
        schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
        generation: generation.clone(),
        id: TemporalRecordId(format!("event-{index:010}")),
        document_id: document(index, "event"),
        source_version: 1,
        at_micros,
    }
}

fn document(index: usize, field: &str) -> IndexDocumentId {
    IndexDocumentId::new(EntityRef::Node(format!("doc-{index}")), field, "0")
}

fn query_point(count: usize, distribution: Distribution) -> i64 {
    event_time(count / 2, distribution)
}

fn sample<E>(
    iterations: usize,
    mut query: impl FnMut() -> Result<u64, E>,
) -> Result<(Vec<Duration>, Vec<u64>), E> {
    let mut latencies = Vec::with_capacity(iterations);
    let mut candidates = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        candidates.push(query()?);
        latencies.push(started.elapsed());
    }
    Ok((latencies, candidates))
}

fn latencies(values: &[Duration]) -> serde_json::Value {
    let mut micros = values
        .iter()
        .map(|value| value.as_micros())
        .collect::<Vec<_>>();
    micros.sort_unstable();
    json!({"p50":percentile(&micros, 50), "p95":percentile(&micros, 95), "p99":percentile(&micros, 99)})
}

fn candidate_stats(values: &[u64]) -> serde_json::Value {
    json!({
        "average":values.iter().sum::<u64>() as f64 / values.len() as f64,
        "max":values.iter().copied().max().unwrap_or_default(),
    })
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    values[(values.len() - 1) * percentile / 100]
}

fn millis(value: Duration) -> f64 {
    value.as_secs_f64() * 1_000.0
}

fn directory_size(path: &Path) -> std::io::Result<u64> {
    fs::read_dir(path)?.try_fold(0, |total, entry| {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let size = if metadata.is_dir() {
            directory_size(&entry.path())?
        } else {
            metadata.len()
        };
        Ok(total + size)
    })
}

#[cfg(unix)]
fn peak_rss_bytes() -> u64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return 0;
    }
    let rss = unsafe { usage.assume_init() }.ru_maxrss as u64;
    if cfg!(target_os = "macos") {
        rss
    } else {
        rss * 1024
    }
}

#[cfg(not(unix))]
fn peak_rss_bytes() -> u64 {
    0
}
