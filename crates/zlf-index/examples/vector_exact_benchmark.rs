use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;
use zlf_core::EntityRef;
use zlf_index::{
    bge_m3_dense_v1, content_fingerprint, ExactVectorStore, GenerationId, IndexDocumentId,
    VectorKey, VectorQuery, VectorRecord, VECTOR_RECORD_SCHEMA_VERSION,
};

const DIMENSION: usize = 64;

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1000".into())
        .parse()?;
    let directory = tempfile::tempdir()?;
    let store = ExactVectorStore::open(directory.path())?;
    let profile = profile();
    let records = (0..count).map(record).collect::<Vec<_>>();
    let started = Instant::now();
    store.apply(&records, &[], &profile)?;
    let build = started.elapsed();
    let (warm, quality) = query_sample(&store, &profile, count)?;
    let updates = (count.saturating_sub(100)..count)
        .map(record)
        .collect::<Vec<_>>();
    let update_started = Instant::now();
    store.apply(&updates, &[], &profile)?;
    let update = update_started.elapsed();
    drop(store);
    let reopened = ExactVectorStore::open(directory.path())?;
    let (fresh, reopened_quality) = query_sample(&reopened, &profile, count)?;
    assert_eq!(quality, reopened_quality);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"zlf-vector-exact-benchmark-v1",
            "documents":count,
            "dimension":DIMENSION,
            "build_ms":milliseconds(build),
            "build_docs_per_second":count as f64 / build.as_secs_f64(),
            "update_docs":updates.len(),
            "update_ms":milliseconds(update),
            "warm_query_us":latencies(&warm),
            "fresh_reader_query_us":latencies(&fresh),
            "quality":quality,
            "peak_rss_bytes":peak_rss_bytes(),
            "index_bytes":directory_size(directory.path())?,
            "top_k":10,
        }))?
    );
    Ok(())
}

fn query_sample(
    store: &ExactVectorStore,
    profile: &zlf_index::EmbeddingModelProfile,
    count: usize,
) -> Result<(Vec<Duration>, serde_json::Value), Box<dyn std::error::Error>> {
    let sample = count.min(100);
    let mut durations = Vec::with_capacity(sample);
    let mut reciprocal_rank = 0.0;
    let mut recall = 0.0;
    for expected in 0..sample {
        let started = Instant::now();
        let hits = store.search(&query(vector(expected)), profile)?;
        durations.push(started.elapsed());
        if let Some(rank) = hits
            .iter()
            .position(|hit| hit.key.document_id.entity.id() == format!("doc-{expected}"))
        {
            reciprocal_rank += 1.0 / (rank + 1) as f64;
            recall += 1.0;
        }
    }
    Ok((
        durations,
        json!({
            "queries":sample,
            "mrr":reciprocal_rank / sample as f64,
            "recall_at_10":recall / sample as f64,
        }),
    ))
}

fn profile() -> zlf_index::EmbeddingModelProfile {
    let mut profile = bge_m3_dense_v1();
    profile.dimension = DIMENSION;
    profile
}

fn record(index: usize) -> VectorRecord {
    let profile = profile();
    VectorRecord {
        schema_version: VECTOR_RECORD_SCHEMA_VERSION,
        key: key(index),
        source_version: 1,
        content_fingerprint: content_fingerprint(&format!("doc-{index}")),
        model_revision: profile.model_revision,
        metric: profile.metric,
        normalized: true,
        values: vector(index),
        metadata: BTreeMap::new(),
    }
}

fn query(values: Vec<f32>) -> VectorQuery {
    VectorQuery {
        generation: GenerationId("benchmark-v1".into()),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        values,
        top_k: 10,
        threshold: None,
        include_sources: Vec::new(),
        exclude_sources: Vec::new(),
        include_entities: Vec::new(),
        exclude_entities: Vec::new(),
        fields: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

fn key(index: usize) -> VectorKey {
    VectorKey {
        generation: GenerationId("benchmark-v1".into()),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        document_id: IndexDocumentId::new(EntityRef::Node(format!("doc-{index}")), "body", "0"),
    }
}

fn vector(seed: usize) -> Vec<f32> {
    let mut state = (seed as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut values = (0..DIMENSION)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as f64 / u64::MAX as f64 * 2.0 - 1.0) as f32
        })
        .collect::<Vec<_>>();
    let norm = values
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    values
        .iter_mut()
        .for_each(|value| *value = (f64::from(*value) / norm) as f32);
    values
}

fn latencies(values: &[Duration]) -> serde_json::Value {
    let mut micros = values
        .iter()
        .map(|value| value.as_micros())
        .collect::<Vec<_>>();
    micros.sort_unstable();
    json!({"p50":percentile(&micros, 50), "p95":percentile(&micros, 95), "p99":percentile(&micros, 99)})
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    values[(values.len() - 1) * percentile / 100]
}

fn milliseconds(value: Duration) -> f64 {
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
