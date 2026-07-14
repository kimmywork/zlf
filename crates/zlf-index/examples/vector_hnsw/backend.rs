use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use hnsw_rs::prelude::{AnnT, DistCosine, Hnsw};
use serde_json::json;

use super::support::{read_queries, Manifest};

pub const BASENAME: &str = "frozen-hnsw";
const MAPPING: &str = "canonical-ids.txt";

#[derive(Clone, Copy)]
pub struct Parameters {
    pub connections: usize,
    pub ef_construction: usize,
    pub max_layer: usize,
}

#[allow(clippy::too_many_lines)]
pub fn build_if_needed(
    root: &Path,
    backend: &Path,
    manifest: &Manifest,
    parameters: Parameters,
) -> Result<(f64, bool), Box<dyn std::error::Error>> {
    let identity = json!({
        "schema":manifest.schema,"files":manifest.files,"dimension":manifest.dimension,
        "generation":"frozen-100k-1024-v1","model_profile":"bge_m3_dense_v1",
        "model_version":1,"mapping":"line-number-to-canonical-document-id-v1",
        "connections":parameters.connections,"ef_construction":parameters.ef_construction,
        "max_layer":parameters.max_layer,
    });
    if let Some(build_ms) = reusable(backend, manifest.document_count, &identity)? {
        return Ok((build_ms, true));
    }
    let temporary = backend.with_extension(format!("building-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary)?;
    }
    fs::create_dir_all(&temporary)?;
    let documents = read_queries(
        &root.join("documents.f32le"),
        manifest.document_count,
        manifest.dimension,
    )?;
    let started = Instant::now();
    let mut hnsw = Hnsw::<f32, DistCosine>::new(
        parameters.connections,
        manifest.document_count,
        parameters.max_layer,
        parameters.ef_construction,
        DistCosine {},
    );
    let refs = documents
        .iter()
        .enumerate()
        .map(|(id, values)| (values, id))
        .collect::<Vec<_>>();
    hnsw.parallel_insert(&refs);
    hnsw.set_searching_mode(true);
    hnsw.file_dump(&temporary, BASENAME)?;
    let build_ms = started.elapsed().as_secs_f64() * 1000.0;
    write_mapping(&temporary.join(MAPPING), manifest.document_count)?;
    fs::write(temporary.join("build-ms.json"), build_ms.to_string())?;
    fs::write(
        temporary.join("dataset.json"),
        serde_json::to_vec_pretty(&identity)?,
    )?;
    if backend.exists() {
        fs::remove_dir_all(backend)?;
    }
    fs::rename(&temporary, backend)?;
    Ok((build_ms, false))
}

pub fn load_mapping(
    backend: &Path,
    count: usize,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let lines = BufReader::new(File::open(backend.join(MAPPING))?)
        .lines()
        .collect::<Result<Vec<_>, _>>()?;
    if lines.len() != count
        || lines
            .iter()
            .enumerate()
            .any(|(id, value)| value != &format!("doc-{id:06}"))
    {
        return Err("invalid HNSW canonical ID mapping".into());
    }
    Ok(lines)
}

fn reusable(
    backend: &Path,
    count: usize,
    identity: &serde_json::Value,
) -> Result<Option<f64>, Box<dyn std::error::Error>> {
    if !backend.exists() {
        return Ok(None);
    }
    let marker = backend.join("dataset.json");
    let build_ms = backend.join("build-ms.json");
    if !marker.is_file() || !build_ms.is_file() {
        return Ok(None);
    }
    if serde_json::from_slice::<serde_json::Value>(&fs::read(marker)?)? != *identity {
        return Err("HNSW backend identity mismatch".into());
    }
    load_mapping(backend, count)?;
    for suffix in ["graph", "data"] {
        if !backend.join(format!("{BASENAME}.hnsw.{suffix}")).is_file() {
            return Err("incomplete HNSW backend publication".into());
        }
    }
    Ok(Some(fs::read_to_string(build_ms)?.parse()?))
}

fn write_mapping(path: &Path, count: usize) -> Result<(), std::io::Error> {
    let mut output = BufWriter::new(File::create(path)?);
    for id in 0..count {
        writeln!(output, "doc-{id:06}")?;
    }
    output.flush()
}
