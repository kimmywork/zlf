#[path = "vector_hnsw/backend.rs"]
mod backend;
#[path = "vector_hnsw/lifecycle.rs"]
mod lifecycle;
#[path = "vector_frozen/support.rs"]
mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Instant;

use backend::{build_if_needed, load_mapping, Parameters, BASENAME};
use hnsw_rs::prelude::{DistCosine, FilterT, Hnsw, HnswIo};
use serde_json::json;
use support::{
    directory_size, latency, manifest, peak_rss_bytes, read_queries, read_u16, read_u32,
};
use zlf_index::{bge_m3_dense_v1, ExactVectorStore, GenerationId, VectorQuery};

const GENERATION: &str = "frozen-100k-1024-v1";

#[derive(Clone, Copy)]
enum FilterKind {
    None,
    Group10,
    Group100,
}

struct GroupFilter<'a> {
    groups: &'a [u16],
    modulo: u16,
    expected: u16,
}
impl FilterT for GroupFilter<'_> {
    fn hnsw_filter(&self, id: &usize) -> bool {
        self.groups[*id] % self.modulo == self.expected
    }
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "data/benchmarks/vector-search-100k-1024-v1".into()),
    );
    let query_limit = usize_arg(2, 100)?;
    let connections = usize_arg(3, 48)?;
    let ef_construction = usize_arg(4, 400)?;
    let manifest = manifest(&root)?;
    if manifest.dimension != 1024
        || manifest.document_count != 100_000
        || !manifest.normalized
        || manifest.metric != "cosine"
        || query_limit == 0
        || query_limit > manifest.query_count
    {
        return Err("incompatible frozen vector dataset".into());
    }
    let groups = read_u16(&root.join("document-groups.u16le"), manifest.document_count)?;
    let queries = read_queries(&root.join("queries.f32le"), query_limit, manifest.dimension)?;
    let self_ids = read_u32(
        &root.join("self-query-document-ids.u32le"),
        manifest.self_query_count,
    )?;
    let exact = ExactVectorStore::open(root.join("backends/exact-rocksdb-v1"))?;
    let exact_started = Instant::now();
    let ground_truth = exact_ground_truth(&exact, &queries, &groups)?;
    let exact_oracle_ms = exact_started.elapsed().as_secs_f64() * 1000.0;
    drop(exact);
    let backend = root.join(format!(
        "backends/hnsw-rs-m{connections}-efc{ef_construction}-v1"
    ));
    let parameters = Parameters {
        connections,
        ef_construction,
        max_layer: 16,
    };
    let (build_ms, reused) = build_if_needed(&root, &backend, &manifest, parameters)?;
    let mut io = HnswIo::new(&backend, BASENAME);
    let load_started = Instant::now();
    let hnsw: Hnsw<f32, DistCosine> = io.load_hnsw()?;
    let reopen_ms = load_started.elapsed().as_secs_f64() * 1000.0;
    let canonical_ids = load_mapping(&backend, manifest.document_count)?;
    let lifecycle = lifecycle::probe(&backend)?;
    if lifecycle["passed"] != true {
        return Err("HNSW immutable rebuild lifecycle probe failed".into());
    }
    let mut results = serde_json::Map::new();
    let mut self_top1 = 0;
    for ef in [128, 256, 512, 1024, 2048] {
        for (name, top_k, filter) in workloads() {
            let truth = ground_truth.get(name).unwrap();
            let mut durations = Vec::with_capacity(query_limit);
            let mut recall = 0.0;
            let mut returned = 0;
            for (index, values) in queries.iter().enumerate() {
                let started = Instant::now();
                let hits = search(&hnsw, values, top_k, ef, filter, &groups, index);
                durations.push(started.elapsed());
                returned += hits.len();
                recall += recall_at(&hits, &truth[index], top_k);
                if ef == 2048
                    && name == "unfiltered_top10"
                    && index < self_ids.len()
                    && hits.first().is_some_and(|id| {
                        canonical_ids[*id] == format!("doc-{:06}", self_ids[index])
                    })
                {
                    self_top1 += 1;
                }
            }
            results.insert(
                format!("ef{ef}_{name}"),
                json!({
                    "latency":latency(&durations), "recall":recall/query_limit as f64,
                    "returned_total":returned,
                }),
            );
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"zlf-vector-frozen-hnsw-v1","documents":manifest.document_count,
            "queries":query_limit,"dimension":manifest.dimension,"dataset_files":manifest.files,
            "backend":"hnsw_rs-0.3.4","parameters":{"connections":parameters.connections,
                "ef_construction":parameters.ef_construction,"max_layer":parameters.max_layer},
            "build_ms":build_ms,"backend_reused":reused,"reopen_ms":reopen_ms,
            "exact_oracle_ms":exact_oracle_ms,"results":results,"lifecycle":lifecycle,
            "canonical_id_mapping":{"schema":"line-number-to-canonical-document-id-v1",
                "entries":canonical_ids.len(),"validated":true},
            "self_queries_checked":self_ids.len().min(query_limit),"self_top1_correct":self_top1,
            "index_bytes":directory_size(&backend)?,"peak_rss_bytes":peak_rss_bytes(),
        }))?
    );
    Ok(())
}

fn usize_arg(index: usize, default: usize) -> Result<usize, std::num::ParseIntError> {
    std::env::args()
        .nth(index)
        .unwrap_or_else(|| default.to_string())
        .parse()
}

fn workloads() -> [(&'static str, usize, FilterKind); 6] {
    [
        ("unfiltered_top10", 10, FilterKind::None),
        ("unfiltered_top100", 100, FilterKind::None),
        ("filter10_top10", 10, FilterKind::Group10),
        ("filter10_top100", 100, FilterKind::Group10),
        ("filter1_top10", 10, FilterKind::Group100),
        ("filter1_top100", 100, FilterKind::Group100),
    ]
}

fn search(
    hnsw: &Hnsw<f32, DistCosine>,
    values: &[f32],
    top_k: usize,
    ef: usize,
    kind: FilterKind,
    groups: &[u16],
    index: usize,
) -> Vec<usize> {
    match kind {
        FilterKind::None => hnsw.search(values, top_k, ef),
        FilterKind::Group10 => {
            let filter = GroupFilter {
                groups,
                modulo: 10,
                expected: index as u16 % 10,
            };
            hnsw.search_filter(values, top_k, ef, Some(&filter))
        }
        FilterKind::Group100 => {
            let filter = GroupFilter {
                groups,
                modulo: 100,
                expected: index as u16 % 100,
            };
            hnsw.search_filter(values, top_k, ef, Some(&filter))
        }
    }
    .into_iter()
    .map(|hit| hit.d_id)
    .collect()
}

type GroundTruth = BTreeMap<&'static str, Vec<Vec<usize>>>;

#[allow(clippy::too_many_lines)]
fn exact_ground_truth(
    store: &ExactVectorStore,
    queries: &[Vec<f32>],
    groups: &[u16],
) -> Result<GroundTruth, Box<dyn std::error::Error>> {
    let profile = bge_m3_dense_v1();
    let mut output = BTreeMap::new();
    for (name, top_k, filter) in workloads() {
        let mut answers = Vec::new();
        for (index, values) in queries.iter().enumerate() {
            let metadata = match filter {
                FilterKind::None => BTreeMap::new(),
                FilterKind::Group10 => {
                    BTreeMap::from([("group_10".into(), format!("{:03}", index % 10))])
                }
                FilterKind::Group100 => {
                    BTreeMap::from([("group_100".into(), format!("{:03}", index % 100))])
                }
            };
            let hits = store.search(
                &VectorQuery {
                    generation: GenerationId(GENERATION.into()),
                    model_profile: "bge_m3_dense_v1".into(),
                    model_version: 1,
                    values: values.clone(),
                    top_k,
                    threshold: None,
                    include_sources: vec![],
                    exclude_sources: vec![],
                    include_entities: vec![],
                    exclude_entities: vec![],
                    fields: vec!["body".into()],
                    metadata,
                },
                &profile,
            )?;
            let ids = hits
                .into_iter()
                .map(|hit| {
                    hit.key
                        .document_id
                        .entity
                        .id()
                        .strip_prefix("doc-")
                        .unwrap()
                        .parse()
                })
                .collect::<Result<Vec<usize>, _>>()?;
            assert!(ids.iter().all(|id| match filter {
                FilterKind::None => true,
                FilterKind::Group10 => groups[*id] % 10 == index as u16 % 10,
                FilterKind::Group100 => groups[*id] % 100 == index as u16 % 100,
            }));
            answers.push(ids);
        }
        output.insert(name, answers);
    }
    Ok(output)
}

fn recall_at(found: &[usize], expected: &[usize], top_k: usize) -> f64 {
    let set = expected
        .iter()
        .take(top_k)
        .copied()
        .collect::<BTreeSet<_>>();
    found
        .iter()
        .take(top_k)
        .filter(|id| set.contains(id))
        .count() as f64
        / top_k as f64
}
