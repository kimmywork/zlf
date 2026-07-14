use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct Manifest {
    pub schema: String,
    pub dimension: usize,
    pub document_count: usize,
    pub query_count: usize,
    pub self_query_count: usize,
    pub normalized: bool,
    pub metric: String,
    pub files: std::collections::BTreeMap<String, String>,
}

pub fn manifest(root: &Path) -> Result<Manifest, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&fs::read(
        root.join("manifest.json"),
    )?)?)
}

pub fn read_f32_batch(
    reader: &mut BufReader<File>,
    count: usize,
    dimension: usize,
) -> std::io::Result<Vec<Vec<f32>>> {
    let mut bytes = vec![0_u8; count * dimension * 4];
    reader.read_exact(&mut bytes)?;
    Ok(bytes
        .chunks_exact(dimension * 4)
        .map(|row| {
            row.chunks_exact(4)
                .map(|value| f32::from_le_bytes(value.try_into().unwrap()))
                .collect()
        })
        .collect())
}

pub fn read_queries(path: &Path, count: usize, dimension: usize) -> std::io::Result<Vec<Vec<f32>>> {
    read_f32_batch(&mut BufReader::new(File::open(path)?), count, dimension)
}

pub fn read_u16(path: &Path, count: usize) -> std::io::Result<Vec<u16>> {
    let mut bytes = vec![0_u8; count * 2];
    File::open(path)?.read_exact(&mut bytes)?;
    Ok(bytes
        .chunks_exact(2)
        .map(|value| u16::from_le_bytes(value.try_into().unwrap()))
        .collect())
}

pub fn read_u32(path: &Path, count: usize) -> std::io::Result<Vec<u32>> {
    let mut bytes = vec![0_u8; count * 4];
    File::open(path)?.read_exact(&mut bytes)?;
    Ok(bytes
        .chunks_exact(4)
        .map(|value| u32::from_le_bytes(value.try_into().unwrap()))
        .collect())
}

pub fn latency(values: &[Duration]) -> serde_json::Value {
    let mut micros = values.iter().map(Duration::as_micros).collect::<Vec<_>>();
    micros.sort_unstable();
    json!({
        "p50_us": percentile(&micros, 50), "p95_us": percentile(&micros, 95),
        "p99_us": percentile(&micros, 99), "mean_us":micros.iter().sum::<u128>() as f64 / micros.len() as f64,
        "queries":micros.len(), "qps":micros.len() as f64 / values.iter().sum::<Duration>().as_secs_f64(),
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
