use std::time::Duration;

use zlf_core::{Result, ZlfError};
use zlf_index::{GenerationId, GenerationMetadata, IndexStatus, IndexWaitResult};

use crate::{GenerationManager, ZlfDatabase};

impl ZlfDatabase {
    pub fn create_generation(&self, metadata: &GenerationMetadata) -> Result<()> {
        GenerationManager::new(&self.storage).create(metadata)
    }

    pub fn generation_action(
        &self,
        target: &str,
        id: &GenerationId,
        action: &str,
        checkpoint: Option<u64>,
        document_count: Option<u64>,
        detail: Option<&str>,
    ) -> Result<Option<u64>> {
        let manager = GenerationManager::new(&self.storage);
        match action {
            "start" => manager.start_build(target, id).map(|_| None),
            "checkpoint" => manager
                .checkpoint(target, id, checkpoint.unwrap_or_default())
                .map(|_| None),
            "validate" => manager.begin_validation(target, id).map(|_| None),
            "validation_passed" => manager
                .validation_passed(
                    target,
                    id,
                    document_count.unwrap_or_default(),
                    detail.unwrap_or_default(),
                )
                .map(|_| None),
            "fail" => manager
                .fail(target, id, detail.unwrap_or_default())
                .map(|_| None),
            "activate" => manager.activate(target, id).map(Some),
            "rollback" => manager.rollback(target, id).map(Some),
            _ => Err(ZlfError::Internal("unknown generation action".into())),
        }
    }

    pub fn index_status(&self, target: &str) -> Result<IndexStatus> {
        GenerationManager::new(&self.storage).status(target)
    }

    pub fn generations(&self, target: &str) -> Result<Vec<GenerationMetadata>> {
        GenerationManager::new(&self.storage).list(target)
    }

    pub fn wait_for_indexes(
        &self,
        targets: &[String],
        minimum_sequence: u64,
        timeout: Duration,
    ) -> Result<IndexWaitResult> {
        crate::index_wait::wait_for_indexes(&self.storage, targets, minimum_sequence, timeout)
    }
}
