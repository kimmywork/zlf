use std::fs;
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

pub fn load_jsonl(path: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn latency_report(values: &[Duration]) -> Value {
    let mut micros = values.iter().map(Duration::as_micros).collect::<Vec<_>>();
    micros.sort_unstable();
    json!({
        "p50_us": percentile(&micros, 50),
        "p95_us": percentile(&micros, 95),
        "p99_us": percentile(&micros, 99),
    })
}

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
