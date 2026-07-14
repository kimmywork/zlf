use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};
use zlf_prolog::bulk_pack::{compile_fact_files, load_fact_pack, BulkCompileOptions};
use zlf_query::ZlfDatabase;
use zlf_storage::Storage;

pub fn load_jsonl(path: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()?)
}

#[allow(clippy::too_many_lines)]
pub fn load_graph_database(
    root: &Path,
    users: &[Value],
    documents: &[Value],
) -> Result<ZlfDatabase, Box<dyn std::error::Error>> {
    fs::create_dir_all(root)?;
    let source = root.join("enterprise-kb.pl");
    let mut writer = BufWriter::new(fs::File::create(&source)?);
    for user in users {
        writeln!(
            writer,
            "node('{}', [user], {{group: \"{}\"}}).",
            user["_id"].as_str().unwrap(),
            user["group"].as_str().unwrap()
        )?;
    }
    for document in documents {
        writeln!(
            writer,
            "node('{}', [document], {{access_group: \"{}\", active: {}, body: {}, valid_from: \"{}\", valid_to: \"{}\"}}).",
            document["_id"].as_str().unwrap(),
            document["access_group"].as_str().unwrap(),
            document["active"].as_bool().unwrap(),
            serde_json::to_string(document["body"].as_str().unwrap())?,
            document["valid_from"].as_str().unwrap(),
            document["valid_to"].as_str().unwrap()
        )?;
    }
    writer.flush()?;
    let pack = root.join("enterprise-kb-pack");
    compile_fact_files(&[source], &pack, &BulkCompileOptions::default())?;
    let graph = root.join("graph");
    let storage = Storage::open(graph.join("storage"))?;
    load_fact_pack(&storage, &pack, 50_000)?;
    drop(storage);
    fs::remove_dir_all(pack)?;
    ZlfDatabase::open_existing(graph).map_err(Into::into)
}

#[allow(dead_code)]
pub fn latency_report(values: &[Duration]) -> Value {
    let mut micros = values.iter().map(Duration::as_micros).collect::<Vec<_>>();
    micros.sort_unstable();
    json!({
        "p50_us": percentile(&micros, 50),
        "p95_us": percentile(&micros, 95),
        "p99_us": percentile(&micros, 99),
    })
}

#[allow(dead_code)]
fn percentile(values: &[u128], percentile: usize) -> u128 {
    values[(values.len() - 1) * percentile / 100]
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
