use std::path::Path;

use serde::{Deserialize, Serialize};
use zlf_core::{Result, ZlfError};
use zlf_storage::{RawMutation, Storage, StorageRecordPlan, STORAGE_KEY_VERSION};

use super::format::{
    read_manifest, BulkPackManifest, RecordReader, BULK_PACK_VERSION, RECORDS_FILE,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkLoadReport {
    pub records_written: u64,
    pub batches_written: u64,
    pub already_loaded: bool,
}

pub fn load_fact_pack(
    storage: &Storage,
    pack: &Path,
    batch_records: usize,
) -> Result<BulkLoadReport> {
    if batch_records == 0 {
        return Err(ZlfError::Internal(
            "bulk batch size must be greater than zero".to_string(),
        ));
    }
    let manifest = read_manifest(pack).map_err(internal)?;
    validate_manifest(&manifest)?;
    validate_records(pack, &manifest)?;
    let marker = completion_marker(&manifest);
    if !storage.scan_prefix(&marker)?.is_empty() {
        return Ok(BulkLoadReport {
            records_written: 0,
            batches_written: 0,
            already_loaded: true,
        });
    }
    let progress_key = progress_marker(&manifest);
    let progress = load_progress(storage, &progress_key)?;
    let (records_written, batches_written) =
        write_records(storage, pack, batch_records, progress, &progress_key)?;
    storage.write_raw_batch(&[
        RawMutation::Put(marker.into_bytes(), b"complete".to_vec()),
        RawMutation::Delete(progress_key.into_bytes()),
    ])?;
    Ok(BulkLoadReport {
        records_written,
        batches_written,
        already_loaded: false,
    })
}

fn write_records(
    storage: &Storage,
    pack: &Path,
    batch_size: usize,
    skip: u64,
    progress_key: &str,
) -> Result<(u64, u64)> {
    let mut reader = RecordReader::open(&pack.join(RECORDS_FILE)).map_err(internal)?;
    let mut plan = StorageRecordPlan::default();
    let mut counts = (0_u64, 0_u64);
    let mut seen = 0_u64;
    while let Some(record) = reader.next_record().map_err(internal)? {
        seen += 1;
        if seen <= skip {
            continue;
        }
        plan.records.push(record);
        if plan.records.len() >= batch_size {
            flush_plan(storage, &mut plan, &mut counts)?;
            store_progress(storage, progress_key, seen)?;
        }
    }
    if !plan.records.is_empty() {
        flush_plan(storage, &mut plan, &mut counts)?;
        store_progress(storage, progress_key, seen)?;
    }
    Ok(counts)
}

fn flush_plan(
    storage: &Storage,
    plan: &mut StorageRecordPlan,
    counts: &mut (u64, u64),
) -> Result<()> {
    counts.0 += storage.write_record_plans([&*plan])? as u64;
    counts.1 += 1;
    plan.records.clear();
    Ok(())
}

fn validate_manifest(manifest: &BulkPackManifest) -> Result<()> {
    if !manifest.complete {
        return Err(invalid("bulk pack is incomplete"));
    }
    if manifest.format_version != BULK_PACK_VERSION {
        return Err(invalid("unsupported bulk pack format version"));
    }
    if manifest.storage_key_version != STORAGE_KEY_VERSION {
        return Err(invalid("bulk pack storage key version mismatch"));
    }
    Ok(())
}

fn validate_records(pack: &Path, manifest: &BulkPackManifest) -> Result<()> {
    let mut reader = RecordReader::open(&pack.join(RECORDS_FILE)).map_err(internal)?;
    let mut count = 0_u64;
    while let Some(record) = reader.next_record().map_err(internal)? {
        validate_record_key(&record.key)?;
        count += 1;
    }
    if count != manifest.record_count {
        return Err(invalid("bulk pack record count mismatch"));
    }
    if reader.checksum() != manifest.records_checksum {
        return Err(invalid("bulk pack record checksum mismatch"));
    }
    Ok(())
}

fn validate_record_key(key: &[u8]) -> Result<()> {
    const PREFIXES: &[&[u8]] = &[b"node:", b"edge:", b"ver:", b"idx:", b"meta:predicate:"];
    if PREFIXES.iter().any(|prefix| key.starts_with(prefix)) {
        Ok(())
    } else {
        Err(invalid("bulk pack contains a forbidden storage key"))
    }
}

fn progress_marker(manifest: &BulkPackManifest) -> String {
    format!("meta:bulk_progress:{:016x}", manifest.records_checksum)
}

fn load_progress(storage: &Storage, key: &str) -> Result<u64> {
    storage.get_raw(key)?.map_or(Ok(0), |bytes| {
        bytes
            .as_slice()
            .try_into()
            .map(u64::from_le_bytes)
            .map_err(|_| invalid("invalid bulk progress marker"))
    })
}

fn store_progress(storage: &Storage, key: &str, records: u64) -> Result<()> {
    storage.put_raw(key, &records.to_le_bytes())
}

fn completion_marker(manifest: &BulkPackManifest) -> String {
    format!("meta:bulk_pack:{:016x}", manifest.records_checksum)
}

fn invalid(message: &str) -> ZlfError {
    ZlfError::Internal(message.to_string())
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}
