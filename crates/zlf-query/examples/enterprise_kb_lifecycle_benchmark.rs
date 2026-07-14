#[path = "enterprise_kb_h6/support.rs"]
mod common;
#[path = "enterprise_kb_lifecycle/support.rs"]
mod support;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde_json::json;
use support::{node, DeterministicProvider, FailOnceProvider};
use zlf_core::Value;
use zlf_index::GenerationState;
use zlf_query::ZlfDatabase;

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "data/benchmarks/enterprise-kb/v1-1k".into()),
    );
    let documents = common::load_jsonl(&dataset.join("documents.jsonl"))?;
    let users = common::load_jsonl(&dataset.join("users.jsonl"))?;
    let mutations = common::load_jsonl(&dataset.join("mutations.jsonl"))?;
    let directory = tempfile::tempdir()?;
    let database_root = directory.path().join("database");
    let build_started = Instant::now();
    let database = common::load_graph_database(&database_root, &users, &documents)?;
    database.put_index_profile(&support::profile())?;
    database.activate_index_profile("enterprise", 1)?;
    let initial_generation = active_generation(&database)?;
    let initial_build = build_started.elapsed();

    let retry_started = Instant::now();
    let retry_time = Utc::now();
    assert_eq!(
        database
            .process_embedding_batch(&FailOnceProvider::new(), retry_time)
            .await?,
        0
    );
    let after_failure = database.embedding_job_state_counts()?;
    let retry_jobs = count(&after_failure, "retry");
    let provider = DeterministicProvider;
    let mut initial_published = 0;
    loop {
        let published = database
            .process_embedding_batch(&provider, retry_time + chrono::Duration::hours(1))
            .await?;
        if published == 0 {
            break;
        }
        initial_published += published;
    }
    let initial_embedding = retry_started.elapsed();

    let mutation_started = Instant::now();
    let mut revised = 0;
    let mut deleted = 0;
    let mut inserted = 0;
    for mutation in &mutations {
        match mutation["kind"].as_str().unwrap() {
            "revise" => {
                let document = &mutation["document"];
                database.set_node_property(
                    document["_id"].as_str().unwrap(),
                    "body",
                    Value::String(document["body"].as_str().unwrap().into()),
                )?;
                revised += 1;
            }
            "delete" => {
                database.query_prolog(&format!(
                    "? retract(node('{}')).",
                    mutation["_id"].as_str().unwrap()
                ))?;
                deleted += 1;
            }
            "insert" => {
                database.add_node(node(&mutation["document"]))?;
                inserted += 1;
            }
            other => return Err(format!("unknown mutation: {other}").into()),
        }
    }
    let final_receipt = database.set_node_property(
        "user-00",
        "benchmark_marker",
        Value::String("complete".into()),
    )?;
    let minimum = final_receipt.sequence.unwrap();
    let wait = database.wait_for_indexes(
        &["bm25".into(), "vector".into(), "temporal".into()],
        minimum,
        Duration::from_secs(5),
    )?;
    let timeout = database.wait_for_indexes(&["missing-target".into()], minimum, Duration::ZERO)?;
    let mut mutation_published = 0;
    loop {
        let published = database
            .process_embedding_batch(&provider, Utc::now() + chrono::Duration::hours(2))
            .await?;
        if published == 0 {
            break;
        }
        mutation_published += published;
    }
    let mutation_latency = mutation_started.elapsed();
    let job_counts = database.embedding_job_state_counts()?;
    let correctness = verify_mutations(&database, &mutations)?;

    let rebuilt_generation = database.rebuild_bm25_generation()?;
    database.rollback_bm25_generation(&initial_generation)?;
    let rollback_active = active_generation(&database)? == initial_generation;
    drop(database);
    let reopen_started = Instant::now();
    let reopened = ZlfDatabase::open_existing(database_root.join("graph"))?;
    let reopen_latency = reopen_started.elapsed();
    let reopen_correctness = verify_mutations(&reopened, &mutations)?;
    let sample = reopened.search_bm25("topic00", 10, &["body".into()], false)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"zlf-enterprise-kb-lifecycle-v1", "dataset":dataset,
            "documents":documents.len(), "mutations":{"revised":revised,"deleted":deleted,"inserted":inserted},
            "phases_ms":{"initial_build":ms(initial_build),"initial_embedding":ms(initial_embedding),
                "mutations_and_embedding":ms(mutation_latency),"reopen":ms(reopen_latency)},
            "embedding":{"injected_retry_jobs":retry_jobs,"initial_published":initial_published,
                "mutation_published":mutation_published,"states":job_counts},
            "watermark":{"minimum":minimum,"reached":wait.reached,"pending":wait.pending_targets,
                "timeout_reached":timeout.reached,"timeout_pending":timeout.pending_targets},
            "generation":{"initial":initial_generation.0,"rebuilt":rebuilt_generation.0,"rollback_active":rollback_active},
            "correctness":{"before_reopen":correctness,"after_reopen":reopen_correctness,
                "bm25_sample_count":sample.len()},
            "created_at":Utc::now(), "peak_rss_bytes":common::peak_rss_bytes(),
            "disk_bytes":common::directory_size(directory.path())?,
        }))?
    );
    Ok(())
}

fn active_generation(
    database: &ZlfDatabase,
) -> Result<zlf_index::GenerationId, Box<dyn std::error::Error>> {
    database
        .generations("bm25")?
        .into_iter()
        .find(|item| item.state == GenerationState::Active)
        .map(|item| item.id)
        .ok_or_else(|| "active BM25 generation missing".into())
}

fn verify_mutations(
    database: &ZlfDatabase,
    mutations: &[serde_json::Value],
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut matched = 0;
    for mutation in mutations {
        let (id, expected) = if mutation["kind"] == "delete" {
            (mutation["_id"].as_str().unwrap(), None)
        } else {
            let document = &mutation["document"];
            (
                document["_id"].as_str().unwrap(),
                Some(document["body"].as_str().unwrap()),
            )
        };
        let node = database.get_node(id)?;
        let valid = match (node, expected) {
            (None, None) => true,
            (Some(node), Some(body)) => {
                node.properties.get("body") == Some(&Value::String(body.into()))
            }
            _ => false,
        };
        matched += usize::from(valid);
    }
    Ok(matched)
}

fn count(counts: &BTreeMap<String, usize>, state: &str) -> usize {
    counts.get(state).copied().unwrap_or(0)
}
fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
