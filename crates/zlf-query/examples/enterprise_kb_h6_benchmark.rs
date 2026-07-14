#[path = "enterprise_kb_h6/support.rs"]
mod support;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use chrono::Utc;
use serde_json::{json, Value as JsonValue};
use support::{directory_size, latency_report, load_jsonl, peak_rss_bytes};
use zlf_core::{EntityRef, Node, Value};
use zlf_index::{
    content_fingerprint, DocumentChanges, GenerationId, IndexDocument, IndexDocumentId,
    TemporalRecordId, ValidityRecord, ValidityStore, INDEX_DOCUMENT_SCHEMA_VERSION,
    TEMPORAL_RECORD_SCHEMA_VERSION,
};
use zlf_query::ZlfDatabase;

const CANDIDATE_LIMIT: usize = 256;
const ANSWER_LIMIT: usize = 10;
const INSTANT: i64 = 1_767_225_600_000_000;

struct FilteredCandidates {
    accepted: Vec<String>,
    scanned: usize,
    temporal_rejected: usize,
    graph_rejected: usize,
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "data/benchmarks/enterprise-kb/v1-1k".into()),
    );
    let documents = load_jsonl(&root.join("documents.jsonl"))?;
    let users = load_jsonl(&root.join("users.jsonl"))?;
    let queries = load_jsonl(&root.join("queries.jsonl"))?;
    let oracle = load_jsonl(&root.join("oracle.jsonl"))?;
    let oracle = oracle
        .into_iter()
        .map(|row| {
            let ids = row["relevant"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap().to_string())
                .collect::<HashSet<_>>();
            (row["query_id"].as_str().unwrap().to_string(), ids)
        })
        .collect::<HashMap<_, _>>();
    let documents_by_id = documents
        .iter()
        .map(|row| (row["_id"].as_str().unwrap(), row))
        .collect::<HashMap<_, _>>();
    let user_groups = users
        .iter()
        .map(|row| (row["_id"].as_str().unwrap(), row["group"].as_str().unwrap()))
        .collect::<HashMap<_, _>>();
    let directory = tempfile::tempdir()?;
    let build_started = Instant::now();
    let database = ZlfDatabase::open(directory.path().join("graph"))?;
    for user in &users {
        database.add_node(Node::with_id(
            user["_id"].as_str().unwrap().into(),
            vec!["user".into()],
            HashMap::from([(
                "group".into(),
                Value::String(user["group"].as_str().unwrap().into()),
            )]),
        ))?;
    }
    for document in &documents {
        database.add_node(graph_node(document))?;
    }
    database.query_prolog("allowed(U,D) :- property(U,group,G), property(D,access_group,G).")?;
    let bm25 = zlf_index::BM25Index::open(directory.path().join("bm25"))?;
    bm25.apply_document_changes(&DocumentChanges {
        upserts: documents.iter().map(index_document).collect(),
        deletes: Vec::new(),
    })?;
    let generation = GenerationId("enterprise-kb-v1".into());
    let validity = ValidityStore::open(directory.path().join("validity"))?;
    let validity_records = documents
        .iter()
        .map(|row| validity_record(row, &generation))
        .collect::<Result<Vec<_>, _>>()?;
    validity.apply(&validity_records, &[])?;
    let build_latency = build_started.elapsed();
    let oracle_started = Instant::now();
    let mut full_rankings = HashMap::new();
    for query in &queries {
        let text = query["text"].as_str().unwrap();
        if !full_rankings.contains_key(text) {
            full_rankings.insert(
                text.to_string(),
                bm25.search_document_top_k(text, documents.len(), &[], &BTreeMap::new(), false)?,
            );
        }
    }
    let oracle_latency = oracle_started.elapsed();
    let mut query_latencies = Vec::new();
    let mut candidates_scanned = 0_usize;
    let mut graph_rejected = 0_usize;
    let mut temporal_rejected = 0_usize;
    let mut answers = 0_usize;
    let mut exact_queries = 0_usize;
    let mut stale_results = 0_usize;
    let mut relevant_answers = 0_usize;
    for query in &queries {
        let started = Instant::now();
        let text = query["text"].as_str().unwrap();
        let ranked =
            bm25.search_document_top_k(text, CANDIDATE_LIMIT, &[], &BTreeMap::new(), false)?;
        let group = user_groups[query["user"].as_str().unwrap()];
        let expected = full_rankings[text]
            .iter()
            .filter(|hit| {
                let document = documents_by_id[hit.document_id.entity.id()];
                document["active"] == true && document["access_group"] == group
            })
            .take(ANSWER_LIMIT)
            .map(|hit| hit.document_id.entity.id().to_string())
            .collect::<Vec<_>>();
        let filtered = filter_candidates(
            &database,
            &validity,
            &generation,
            query["user"].as_str().unwrap(),
            &ranked,
        )?;
        candidates_scanned += filtered.scanned;
        temporal_rejected += filtered.temporal_rejected;
        graph_rejected += filtered.graph_rejected;
        answers += filtered.accepted.len();
        exact_queries += usize::from(filtered.accepted == expected);
        relevant_answers += filtered
            .accepted
            .iter()
            .filter(|id| oracle[query["_id"].as_str().unwrap()].contains(*id))
            .count();
        stale_results += filtered
            .accepted
            .iter()
            .filter(|id| {
                let document = documents_by_id[id.as_str()];
                document["active"] != true || document["access_group"] != group
            })
            .count();
        query_latencies.push(started.elapsed());
    }
    database.query_prolog(":- table allowed/2.")?;
    let old_document = documents
        .iter()
        .find(|row| row["access_group"] == "group-00" && row["active"] == true)
        .unwrap()["_id"]
        .as_str()
        .unwrap();
    let old_was_allowed = is_allowed(&database, "user-00", old_document)?;
    let _ = is_allowed(&database, "user-00", old_document)?;
    let invalidations_before = database.table_metrics().stale_invalidations;
    database.set_node_property("user-00", "group", Value::String("group-01".into()))?;
    let mutation_invalidated = database.table_metrics().stale_invalidations > invalidations_before;
    let old_is_stale = is_allowed(&database, "user-00", old_document)?;
    stale_results += usize::from(!old_was_allowed || old_is_stale);
    let query_count = queries.len().max(1) as f64;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"zlf-enterprise-kb-h6-v1", "dataset":root, "documents":documents.len(), "queries":queries.len(),
            "candidate_limit":CANDIDATE_LIMIT, "answer_limit":ANSWER_LIMIT,
            "build_ms":build_latency.as_secs_f64()*1000.0, "oracle_ms":oracle_latency.as_secs_f64()*1000.0,
            "query_latency":latency_report(&query_latencies),
            "candidate_counts":{"scanned_total":candidates_scanned,"average_per_query":candidates_scanned as f64/query_count,
                "answers_total":answers,"peak_materialized_answers":ANSWER_LIMIT},
            "filter":{"graph_rejected":graph_rejected,"temporal_rejected":temporal_rejected,
                "selection_rate":answers as f64/candidates_scanned.max(1) as f64},
            "quality":{"relevant_answers":relevant_answers,"precision":relevant_answers as f64/answers.max(1) as f64},
            "correctness":{"exact_filtered_top_k_queries":exact_queries,"stale_result_count":stale_results,
                "permission_mutation_invalidated_tables":mutation_invalidated},
            "disk_bytes":directory_size(directory.path())?, "peak_rss_bytes":peak_rss_bytes(), "created_at":Utc::now(),
        }))?
    );
    Ok(())
}

fn graph_node(row: &JsonValue) -> Node {
    Node::with_id(
        row["_id"].as_str().unwrap().into(),
        vec!["document".into()],
        HashMap::from([
            (
                "access_group".into(),
                Value::String(row["access_group"].as_str().unwrap().into()),
            ),
            (
                "active".into(),
                Value::Bool(row["active"].as_bool().unwrap()),
            ),
        ]),
    )
}

fn index_document(row: &JsonValue) -> IndexDocument {
    let text = row["body"].as_str().unwrap();
    IndexDocument {
        schema_version: INDEX_DOCUMENT_SCHEMA_VERSION,
        id: IndexDocumentId::new(
            EntityRef::Node(row["_id"].as_str().unwrap().into()),
            "body",
            "0",
        ),
        source_version: 1,
        content_fingerprint: content_fingerprint(text),
        source_range: None,
        chunk_ordinal: 0,
        chunk_profile: "enterprise-kb-v1".into(),
        language: Some("en".into()),
        content: text.into(),
    }
}

fn validity_record(
    row: &JsonValue,
    generation: &GenerationId,
) -> Result<ValidityRecord, Box<dyn std::error::Error>> {
    Ok(ValidityRecord {
        schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
        generation: generation.clone(),
        id: TemporalRecordId(format!("validity:{}", row["_id"].as_str().unwrap())),
        document_id: IndexDocumentId::new(
            EntityRef::Node(row["_id"].as_str().unwrap().into()),
            "validity",
            "0",
        ),
        source_version: 1,
        valid_from_micros: zlf_index::parse_utc_micros(row["valid_from"].as_str().unwrap())?,
        valid_to_micros: Some(zlf_index::parse_utc_micros(
            row["valid_to"].as_str().unwrap(),
        )?),
    })
}

fn filter_candidates(
    database: &ZlfDatabase,
    validity: &ValidityStore,
    generation: &GenerationId,
    user: &str,
    ranked: &[zlf_index::BM25DocumentHit],
) -> Result<FilteredCandidates, Box<dyn std::error::Error>> {
    let mut accepted = Vec::new();
    let (mut scanned, mut temporal_rejected, mut graph_rejected) = (0, 0, 0);
    for hit in ranked.iter().take(CANDIDATE_LIMIT) {
        scanned += 1;
        if validity
            .valid_at_for_entity(generation, &hit.document_id.entity, INSTANT, 1)?
            .records
            .is_empty()
        {
            temporal_rejected += 1;
            continue;
        }
        if !is_allowed(database, user, hit.document_id.entity.id())? {
            graph_rejected += 1;
            continue;
        }
        accepted.push(hit.document_id.entity.id().to_string());
        if accepted.len() == ANSWER_LIMIT {
            break;
        }
    }
    Ok(FilteredCandidates {
        accepted,
        scanned,
        temporal_rejected,
        graph_rejected,
    })
}

fn is_allowed(
    database: &ZlfDatabase,
    user: &str,
    document: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let query = format!("? allowed('{user}', '{document}').");
    Ok(!database.query_prolog(&query)?.is_empty())
}
