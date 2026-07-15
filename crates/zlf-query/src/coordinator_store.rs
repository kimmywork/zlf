use zlf_core::{Result, ZlfError};
use zlf_storage::Storage;

use crate::coordinator::{DurableIndexJob, TargetProgress};

const TARGET_PREFIX: &str = "projection:index-coordinator:target:";
const JOB_PREFIX: &str = "projection:index-coordinator:job:";

pub(crate) fn load_progress(storage: &Storage, target: &str) -> Result<TargetProgress> {
    storage
        .get_raw(&target_key(target))?
        .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
        .transpose()
        .map(|progress| progress.unwrap_or_else(|| TargetProgress::new(target)))
}

pub(crate) fn save_progress(storage: &Storage, progress: &TargetProgress) -> Result<()> {
    storage.put_raw(
        &target_key(&progress.target),
        &bincode::serialize(progress).map_err(serialization)?,
    )
}

pub(crate) fn list_progress(storage: &Storage) -> Result<Vec<TargetProgress>> {
    storage
        .scan_prefix(TARGET_PREFIX)?
        .into_iter()
        .map(|(_, bytes)| bincode::deserialize(&bytes).map_err(serialization))
        .collect()
}

pub(crate) fn save_job(storage: &Storage, job: &DurableIndexJob) -> Result<()> {
    storage.put_raw(
        &job_key(&job.target, job.event_sequence),
        &bincode::serialize(job).map_err(serialization)?,
    )
}

pub(crate) fn load_jobs(storage: &Storage, target: &str) -> Result<Vec<DurableIndexJob>> {
    storage
        .scan_prefix(&job_prefix(target))?
        .into_iter()
        .map(|(_, bytes)| bincode::deserialize(&bytes).map_err(serialization))
        .collect()
}

pub(crate) fn delete_target(storage: &Storage, target: &str) -> Result<()> {
    for (key, _) in storage.scan_prefix(&job_prefix(target))? {
        storage.delete_raw(&key)?;
    }
    storage.delete_raw(&target_key(target))
}

fn target_key(target: &str) -> String {
    format!("{TARGET_PREFIX}{}", hex(target.as_bytes()))
}

fn job_prefix(target: &str) -> String {
    format!("{JOB_PREFIX}{}:", hex(target.as_bytes()))
}

fn job_key(target: &str, sequence: u64) -> String {
    format!("{}{sequence:020}", job_prefix(target))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
