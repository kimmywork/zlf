use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

pub struct Quality {
    reciprocal_rank: f64,
    ndcg_10: f64,
    recall_10: f64,
    recall_100: f64,
    queries: usize,
}

impl Quality {
    pub fn observe(&mut self, ranking: &[String], qrels: &HashMap<String, i32>) {
        self.queries += 1;
        if let Some(rank) = ranking
            .iter()
            .position(|id| qrels.get(id).is_some_and(|score| *score > 0))
        {
            self.reciprocal_rank += 1.0 / (rank + 1) as f64;
        }
        self.ndcg_10 += ndcg(ranking, qrels, 10);
        self.recall_10 += recall(ranking, qrels, 10);
        self.recall_100 += recall(ranking, qrels, 100);
    }

    pub fn report(&self) -> Value {
        let count = self.queries.max(1) as f64;
        json!({
            "mrr":self.reciprocal_rank / count,
            "ndcg_at_10":self.ndcg_10 / count,
            "recall_at_10":self.recall_10 / count,
            "recall_at_100":self.recall_100 / count,
            "queries":self.queries,
        })
    }
}

impl Default for Quality {
    fn default() -> Self {
        Self {
            reciprocal_rank: 0.0,
            ndcg_10: 0.0,
            recall_10: 0.0,
            recall_100: 0.0,
            queries: 0,
        }
    }
}

fn recall(ranking: &[String], qrels: &HashMap<String, i32>, limit: usize) -> f64 {
    let relevant = qrels.values().filter(|score| **score > 0).count();
    if relevant == 0 {
        return 0.0;
    }
    ranking
        .iter()
        .take(limit)
        .filter(|id| qrels.get(*id).is_some_and(|score| *score > 0))
        .count() as f64
        / relevant as f64
}

fn ndcg(ranking: &[String], qrels: &HashMap<String, i32>, limit: usize) -> f64 {
    let dcg = ranking
        .iter()
        .take(limit)
        .enumerate()
        .map(|(rank, id)| gain(*qrels.get(id).unwrap_or(&0), rank))
        .sum::<f64>();
    let mut ideal = qrels.values().copied().collect::<Vec<_>>();
    ideal.sort_unstable_by(|left, right| right.cmp(left));
    let idcg = ideal
        .into_iter()
        .take(limit)
        .enumerate()
        .map(|(rank, score)| gain(score, rank))
        .sum::<f64>();
    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

fn gain(score: i32, rank: usize) -> f64 {
    (2_f64.powi(score) - 1.0) / ((rank + 2) as f64).log2()
}

pub fn latency_report(values: &[Duration]) -> Value {
    let mut micros = values.iter().map(Duration::as_micros).collect::<Vec<_>>();
    micros.sort_unstable();
    json!({
        "p50_us":percentile(&micros, 50),
        "p95_us":percentile(&micros, 95),
        "p99_us":percentile(&micros, 99),
    })
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    values[(values.len() - 1) * percentile / 100]
}

pub fn load_jsonl(path: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn load_qrels(
    path: &Path,
) -> Result<HashMap<String, HashMap<String, i32>>, Box<dyn std::error::Error>> {
    let mut qrels = HashMap::<String, HashMap<String, i32>>::new();
    for line in fs::read_to_string(path)?.lines().skip(1) {
        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.len() == 3 {
            qrels
                .entry(parts[0].into())
                .or_default()
                .insert(parts[1].into(), parts[2].parse()?);
        }
    }
    Ok(qrels)
}

pub fn document_text(row: &Value) -> String {
    format!(
        "{}\n{}",
        row["title"].as_str().unwrap_or_default(),
        row["text"].as_str().unwrap_or_default()
    )
}

pub fn normalize(mut values: Vec<f32>) -> Result<Vec<f32>, String> {
    let norm = values
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    if norm == 0.0 || !norm.is_finite() {
        return Err("embedding is zero or non-finite".into());
    }
    values.iter_mut().for_each(|value| *value /= norm as f32);
    Ok(values)
}

pub fn directory_size(path: &Path) -> std::io::Result<u64> {
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
pub fn peak_rss_bytes() -> u64 {
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
pub fn peak_rss_bytes() -> u64 {
    0
}
