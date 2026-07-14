#[path = "vector_frozen/support.rs"]
mod support;

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::json;
use support::{
    directory_size, latency, manifest, peak_rss_bytes, read_f32_batch, read_queries, read_u16,
    read_u32,
};
use zlf_core::EntityRef;
use zlf_index::{
    bge_m3_dense_v1, content_fingerprint, EmbeddingModelProfile, ExactVectorStore, GenerationId,
    IndexDocumentId, VectorKey, VectorQuery, VectorRecord, VECTOR_RECORD_SCHEMA_VERSION,
};

const GENERATION: &str = "frozen-100k-1024-v1";

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "data/benchmarks/vector-search-100k-1024-v1".into()),
    );
    let query_limit = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "100".into())
        .parse::<usize>()?;
    let manifest = manifest(&root)?;
    validate_manifest(&manifest, query_limit)?;
    let backend = root.join("backends/exact-rocksdb-v1");
    let profile = profile(manifest.dimension);
    let groups = read_u16(&root.join("document-groups.u16le"), manifest.document_count)?;
    let (store, build_ms, reused) = open_or_build(&root, &backend, &manifest, &profile, &groups)?;
    let queries = read_queries(&root.join("queries.f32le"), query_limit, manifest.dimension)?;
    let self_ids = read_u32(
        &root.join("self-query-document-ids.u32le"),
        manifest.self_query_count,
    )?;
    let mut workloads = serde_json::Map::new();
    let mut self_correct = 0;
    for (name, top_k, filter) in [
        ("unfiltered_top10", 10, None),
        ("unfiltered_top100", 100, None),
        ("filter10_top10", 10, Some(("group_10", 10_u16))),
        ("filter10_top100", 100, Some(("group_10", 10_u16))),
        ("filter1_top10", 10, Some(("group_100", 100_u16))),
        ("filter1_top100", 100, Some(("group_100", 100_u16))),
    ] {
        let mut durations = Vec::with_capacity(queries.len());
        let mut returned = 0;
        for (index, values) in queries.iter().enumerate() {
            let metadata = filter.map_or_else(BTreeMap::new, |(key, modulo)| {
                BTreeMap::from([(key.into(), format!("{:03}", index as u16 % modulo))])
            });
            let started = Instant::now();
            let hits = store.search(&query(values.clone(), top_k, metadata), &profile)?;
            durations.push(started.elapsed());
            returned += hits.len();
            if name == "unfiltered_top10"
                && index < self_ids.len()
                && hits.first().is_some_and(|hit| {
                    document_index(hit.key.document_id.entity.id()) == self_ids[index] as usize
                })
            {
                self_correct += 1;
            }
            if let Some((_, modulo)) = filter {
                assert!(hits.iter().all(|hit| groups
                    [document_index(hit.key.document_id.entity.id())]
                    % modulo
                    == index as u16 % modulo));
            }
        }
        workloads.insert(
            name.into(),
            json!({"latency":latency(&durations),"returned_total":returned}),
        );
    }
    drop(store);
    let fresh_started = Instant::now();
    let reopened = ExactVectorStore::open(&backend)?;
    let reopen_ms = fresh_started.elapsed().as_secs_f64() * 1000.0;
    let mut fresh = Vec::new();
    for values in queries.iter().take(10) {
        let started = Instant::now();
        reopened.search(&query(values.clone(), 10, BTreeMap::new()), &profile)?;
        fresh.push(started.elapsed());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"zlf-vector-frozen-exact-v1", "dataset_schema":manifest.schema,
            "documents":manifest.document_count,"queries":query_limit,"dimension":manifest.dimension,
            "metric":manifest.metric,"normalized":manifest.normalized,"dataset_files":manifest.files,
            "backend":"exact-rocksdb-v1","backend_reused":reused,"build_ms":build_ms,
            "workloads":workloads,"self_queries_checked":self_ids.len().min(query_limit),
            "self_top1_correct":self_correct,"reopen_ms":reopen_ms,"fresh_reader_top10":latency(&fresh),
            "index_bytes":directory_size(&backend)?,"peak_rss_bytes":peak_rss_bytes(),
        }))?
    );
    Ok(())
}

fn validate_manifest(
    manifest: &support::Manifest,
    query_limit: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if manifest.schema != "zlf-vector-search-dataset-v1"
        || manifest.dimension != 1024
        || manifest.document_count != 100_000
        || query_limit == 0
        || query_limit > manifest.query_count
        || !manifest.normalized
        || manifest.metric != "cosine"
    {
        return Err("incompatible frozen vector dataset".into());
    }
    Ok(())
}

fn open_or_build(
    root: &Path,
    backend: &Path,
    manifest: &support::Manifest,
    profile: &EmbeddingModelProfile,
    groups: &[u16],
) -> Result<(ExactVectorStore, f64, bool), Box<dyn std::error::Error>> {
    let marker = backend.join("dataset.json");
    let identity =
        json!({"schema":manifest.schema,"files":manifest.files,"dimension":manifest.dimension});
    if marker.is_file() {
        if serde_json::from_slice::<serde_json::Value>(&fs::read(&marker)?)? != identity {
            return Err("exact backend belongs to a different dataset".into());
        }
        return Ok((ExactVectorStore::open(backend)?, 0.0, true));
    }
    if backend.exists() {
        fs::remove_dir_all(backend)?;
    }
    let store = ExactVectorStore::open(backend)?;
    let mut reader = BufReader::new(File::open(root.join("documents.f32le"))?);
    let started = Instant::now();
    for start in (0..manifest.document_count).step_by(256) {
        let count = 256.min(manifest.document_count - start);
        let values = read_f32_batch(&mut reader, count, manifest.dimension)?;
        let records = values
            .into_iter()
            .enumerate()
            .map(|(offset, values)| record(start + offset, groups[start + offset], values, profile))
            .collect::<Vec<_>>();
        store.apply(&records, &[], profile)?;
    }
    fs::write(marker, serde_json::to_vec_pretty(&identity)?)?;
    Ok((store, started.elapsed().as_secs_f64() * 1000.0, false))
}

fn profile(dimension: usize) -> EmbeddingModelProfile {
    let mut profile = bge_m3_dense_v1();
    profile.dimension = dimension;
    profile
}
fn key(index: usize) -> VectorKey {
    VectorKey {
        generation: GenerationId(GENERATION.into()),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        document_id: IndexDocumentId::new(EntityRef::Node(format!("doc-{index:06}")), "body", "0"),
    }
}
fn record(
    index: usize,
    group: u16,
    values: Vec<f32>,
    profile: &EmbeddingModelProfile,
) -> VectorRecord {
    VectorRecord {
        schema_version: VECTOR_RECORD_SCHEMA_VERSION,
        key: key(index),
        source_version: 1,
        content_fingerprint: content_fingerprint(&format!("frozen-doc-{index:06}")),
        model_revision: profile.model_revision.clone(),
        metric: profile.metric,
        normalized: true,
        values,
        metadata: BTreeMap::from([
            ("group_10".into(), format!("{:03}", group % 10)),
            ("group_100".into(), format!("{:03}", group % 100)),
        ]),
    }
}
fn query(values: Vec<f32>, top_k: usize, metadata: BTreeMap<String, String>) -> VectorQuery {
    VectorQuery {
        generation: GenerationId(GENERATION.into()),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        values,
        top_k,
        threshold: None,
        include_sources: Vec::new(),
        exclude_sources: Vec::new(),
        include_entities: Vec::new(),
        exclude_entities: Vec::new(),
        fields: vec!["body".into()],
        metadata,
    }
}
fn document_index(id: &str) -> usize {
    id.strip_prefix("doc-").unwrap().parse().unwrap()
}
