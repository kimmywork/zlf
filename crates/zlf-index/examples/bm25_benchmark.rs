use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;
use zlf_core::EntityRef;
use zlf_index::{
    BM25Index, ContentFingerprint, DocumentChanges, IndexDocument, IndexDocumentId,
    INDEX_DOCUMENT_SCHEMA_VERSION,
};

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1000".into())
        .parse::<usize>()?;
    let directory = tempfile::tempdir()?;
    let index = BM25Index::open(directory.path())?;
    let documents = (0..count).map(document).collect::<Vec<_>>();
    let started = Instant::now();
    index.apply_document_changes(&DocumentChanges {
        upserts: documents,
        deletes: Vec::new(),
    })?;
    let build = started.elapsed();
    let update_count = count.min(100);
    let updates = (count - update_count..count)
        .map(|id| {
            let mut changed = document(id);
            changed.source_version = 2;
            changed.content.push_str(" refreshed");
            changed
        })
        .collect();
    let update_started = Instant::now();
    index.apply_document_changes(&DocumentChanges {
        upserts: updates,
        deletes: Vec::new(),
    })?;
    let update = update_started.elapsed();
    let (warm, quality) = query_sample(&index, count)?;
    drop(index);
    let reopened = BM25Index::open(directory.path())?;
    let (fresh, reopened_quality) = query_sample(&reopened, count)?;
    assert_eq!(quality, reopened_quality);
    let report = json!({
        "schema":"zlf-bm25-benchmark-v1",
        "documents":count,
        "build_ms":milliseconds(build),
        "build_docs_per_second":count as f64 / build.as_secs_f64(),
        "update_docs":update_count,
        "update_ms":milliseconds(update),
        "update_docs_per_second":update_count as f64 / update.as_secs_f64(),
        "warm_query_us":latencies(&warm),
        "fresh_reader_query_us":latencies(&fresh),
        "quality":quality,
        "peak_rss_bytes":peak_rss_bytes(),
        "index_bytes":directory_size(directory.path())?,
        "candidate_limit":10_000,
        "top_k":10,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn document(index: usize) -> IndexDocument {
    IndexDocument {
        schema_version: INDEX_DOCUMENT_SCHEMA_VERSION,
        id: IndexDocumentId::new(EntityRef::Node(format!("doc-{index}")), "body", "0"),
        source_version: 1,
        content_fingerprint: ContentFingerprint(format!("fixture-{index}")),
        source_range: None,
        chunk_ordinal: 0,
        chunk_profile: "benchmark-v1".into(),
        language: Some("en".into()),
        content: format!("common knowledge topic{} answer{index}", index % 100),
    }
}

#[allow(clippy::too_many_lines)]
fn query_sample(
    index: &BM25Index,
    count: usize,
) -> Result<(Vec<Duration>, serde_json::Value), Box<dyn std::error::Error>> {
    let sample = count.min(100);
    let mut durations = Vec::with_capacity(sample);
    let mut reciprocal_rank = 0.0;
    let mut recall_at_10 = 0.0;
    let mut ndcg_at_10 = 0.0;
    for expected in 0..sample {
        let started = Instant::now();
        let hits = index.search_document_top_k(
            &format!("answer{expected}"),
            10,
            &[],
            &BTreeMap::new(),
            false,
        )?;
        durations.push(started.elapsed());
        if let Some(rank) = hits
            .iter()
            .position(|hit| hit.document_id.entity.id() == format!("doc-{expected}"))
        {
            reciprocal_rank += 1.0 / (rank + 1) as f64;
            recall_at_10 += 1.0;
            ndcg_at_10 += 1.0 / ((rank + 2) as f64).log2();
        }
    }
    let denominator = sample as f64;
    Ok((
        durations,
        json!({
            "queries":sample,
            "mrr":reciprocal_rank / denominator,
            "ndcg_at_10":ndcg_at_10 / denominator,
            "recall_at_10":recall_at_10 / denominator,
            "recall_at_100":recall_at_10 / denominator,
        }),
    ))
}

fn latencies(values: &[Duration]) -> serde_json::Value {
    let mut micros = values
        .iter()
        .map(|value| value.as_micros())
        .collect::<Vec<_>>();
    micros.sort_unstable();
    json!({
        "p50":percentile(&micros, 50),
        "p95":percentile(&micros, 95),
        "p99":percentile(&micros, 99),
    })
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
    let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if result != 0 {
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
