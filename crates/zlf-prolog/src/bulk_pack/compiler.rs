use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use zlf_core::{Edge, Node, Result, ZlfError};
use zlf_storage::{Storage, STORAGE_KEY_VERSION};

use crate::parser::{PrologParser, Term};
use crate::wam::fact_lowering::{lower_fact, FactMutation};
use crate::wam::storage_writer::edge_id;

use super::format::{
    checksum_file, write_manifest, BulkPackManifest, RecordWriter, BULK_PACK_VERSION, RECORDS_FILE,
};
use super::statement::StatementReader;

#[derive(Debug, Clone)]
pub struct BulkCompileOptions {
    pub timestamp: DateTime<Utc>,
}

impl Default for BulkCompileOptions {
    fn default() -> Self {
        Self {
            timestamp: Utc.timestamp_opt(0, 0).single().expect("Unix epoch exists"),
        }
    }
}

pub fn compile_fact_files(
    inputs: &[PathBuf],
    output: &Path,
    options: &BulkCompileOptions,
) -> Result<BulkPackManifest> {
    if output.exists() {
        return Err(ZlfError::DatabaseAlreadyExists(
            output.display().to_string(),
        ));
    }
    std::fs::create_dir_all(output).map_err(internal)?;
    let mut writer = RecordWriter::create(&output.join(RECORDS_FILE)).map_err(internal)?;
    let (source_checksums, fact_counts) = compile_inputs(inputs, options, &mut writer)?;
    let (record_count, records_checksum) = writer.finish().map_err(internal)?;
    let manifest = BulkPackManifest {
        format_version: BULK_PACK_VERSION,
        storage_key_version: STORAGE_KEY_VERSION,
        source_checksums,
        fact_counts,
        record_count,
        records_checksum,
        complete: true,
    };
    write_manifest(output, &manifest).map_err(internal)?;
    Ok(manifest)
}

fn compile_inputs(
    inputs: &[PathBuf],
    options: &BulkCompileOptions,
    writer: &mut RecordWriter,
) -> Result<(BTreeMap<String, u64>, BTreeMap<String, u64>)> {
    let mut source_checksums = BTreeMap::new();
    let mut fact_counts = BTreeMap::new();
    let mut metadata_keys = HashSet::new();
    for input in inputs {
        source_checksums.insert(display_path(input), checksum_file(input).map_err(internal)?);
        compile_file(input, options, writer, &mut fact_counts, &mut metadata_keys)?;
    }
    Ok((source_checksums, fact_counts))
}

fn compile_file(
    input: &Path,
    options: &BulkCompileOptions,
    writer: &mut RecordWriter,
    fact_counts: &mut BTreeMap<String, u64>,
    metadata_keys: &mut HashSet<Vec<u8>>,
) -> Result<()> {
    let mut statements = StatementReader::open(input).map_err(internal)?;
    while let Some(source) = statements.next_statement().map_err(internal)? {
        let fact = PrologParser::parse_fact(source.trim())?;
        if !is_ground(&fact.head) {
            return Err(ZlfError::Internal(
                "bulk facts must be ground and cannot contain variables".to_string(),
            ));
        }
        let predicate = predicate_name(&fact.head).to_string();
        let plan = mutation_plan(lower_fact(&fact.head).map_err(internal)?, options)?;
        for record in &plan.records {
            if !record.key.starts_with(b"meta:predicate:")
                || metadata_keys.insert(record.key.clone())
            {
                writer.write(record).map_err(internal)?;
            }
        }
        *fact_counts.entry(predicate).or_insert(0) += 1;
    }
    Ok(())
}

fn mutation_plan(
    mutation: FactMutation,
    options: &BulkCompileOptions,
) -> Result<zlf_storage::StorageRecordPlan> {
    match mutation {
        FactMutation::EnsureNode {
            id,
            labels,
            properties,
        } => Storage::compile_node_records(&Node {
            id,
            labels,
            properties,
            current_version: 1,
            created_at: options.timestamp,
            updated_at: options.timestamp,
        }),
        FactMutation::EnsureEdge {
            source,
            edge_type,
            target,
            properties,
        } => edge_plan(source, edge_type, target, properties, options),
        FactMutation::SetProperty { .. } => Err(ZlfError::Internal(
            "bulk pack v1 requires complete node facts; incremental property facts are unsupported"
                .to_string(),
        )),
    }
}

fn edge_plan(
    source: String,
    edge_type: String,
    target: String,
    properties: std::collections::HashMap<String, zlf_core::Value>,
    options: &BulkCompileOptions,
) -> Result<zlf_storage::StorageRecordPlan> {
    Storage::compile_edge_records(&Edge {
        id: edge_id(&source, &edge_type, &target),
        edge_type,
        source,
        target,
        properties,
        created_at: options.timestamp,
        updated_at: options.timestamp,
    })
}

fn is_ground(term: &Term) -> bool {
    match term {
        Term::Variable(_) => false,
        Term::Compound { args, .. } | Term::List(args) => args.iter().all(is_ground),
        Term::Object(entries) => entries.iter().all(|(_, value)| is_ground(value)),
        _ => true,
    }
}

fn predicate_name(term: &Term) -> &str {
    match term {
        Term::Atom(name) | Term::Compound { name, .. } => name,
        _ => "$invalid",
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}
