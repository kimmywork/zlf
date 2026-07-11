use std::collections::BTreeMap;
use std::sync::Mutex;

use zlf_storage::{MutationEvent, Storage};

use crate::coordinator::{IndexTarget, TargetApplyError};

#[derive(Debug, Clone, Copy)]
pub enum FakeFailureMode {
    RetryBeforeWrite,
    RetryAfterWrite,
    Permanent,
}

pub struct FakeIndexTarget {
    name: String,
    failures: Mutex<BTreeMap<u64, (FakeFailureMode, u32)>>,
}

impl FakeIndexTarget {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            failures: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn fail(&self, sequence: u64, mode: FakeFailureMode, times: u32) {
        self.failures
            .lock()
            .expect("fake target failure lock")
            .insert(sequence, (mode, times));
    }

    pub fn applied_sequences(&self, storage: &Storage) -> zlf_core::Result<Vec<u64>> {
        storage
            .scan_prefix(&self.prefix())?
            .into_iter()
            .map(|(key, _)| {
                key.rsplit_once(':')
                    .and_then(|(_, sequence)| sequence.parse().ok())
                    .ok_or_else(|| {
                        zlf_core::ZlfError::Serialization("invalid fake target key".into())
                    })
            })
            .collect()
    }

    fn prefix(&self) -> String {
        format!("projection:fake-target:{}:", hex(self.name.as_bytes()))
    }

    fn key(&self, sequence: u64) -> String {
        format!("{}{sequence:020}", self.prefix())
    }

    fn take_failure(&self, sequence: u64) -> Option<FakeFailureMode> {
        let mut failures = self.failures.lock().expect("fake target failure lock");
        let (mode, remaining) = failures.get_mut(&sequence)?;
        if *remaining == 0 {
            return None;
        }
        *remaining -= 1;
        Some(*mode)
    }
}

impl IndexTarget for FakeIndexTarget {
    fn apply(
        &self,
        storage: &Storage,
        event: &MutationEvent,
    ) -> std::result::Result<(), TargetApplyError> {
        let key = self.key(event.sequence);
        if storage.get_raw(&key).map_err(storage_error)?.is_some() {
            return Ok(());
        }
        let failure = self.take_failure(event.sequence);
        if matches!(failure, Some(FakeFailureMode::RetryBeforeWrite)) {
            return Err(retry_error());
        }
        if matches!(failure, Some(FakeFailureMode::Permanent)) {
            return Err(TargetApplyError {
                message: "permanent fake target failure".into(),
                retryable: false,
            });
        }
        storage
            .put_raw(
                &key,
                &bincode::serialize(event).map_err(|error| TargetApplyError {
                    message: error.to_string(),
                    retryable: false,
                })?,
            )
            .map_err(storage_error)?;
        if matches!(failure, Some(FakeFailureMode::RetryAfterWrite)) {
            return Err(retry_error());
        }
        Ok(())
    }
}

fn retry_error() -> TargetApplyError {
    TargetApplyError {
        message: "retryable fake target failure".into(),
        retryable: true,
    }
}

fn storage_error(error: impl std::fmt::Display) -> TargetApplyError {
    TargetApplyError {
        message: error.to_string(),
        retryable: true,
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
