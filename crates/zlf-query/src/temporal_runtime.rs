use std::path::Path;

use chrono::Utc;
use zlf_core::Result;
use zlf_index::{
    EventTimeStore, GenerationId, GenerationMetadata, GenerationState, ValidityStore,
    GENERATION_SCHEMA_VERSION,
};
use zlf_storage::Storage;

use crate::{
    CoordinatorConfig, GenerationManager, IndexCoordinator, TemporalIndexTarget, ZlfDatabase,
};

const TARGET: &str = "temporal";

pub(crate) struct TemporalRuntimeParts {
    pub events: EventTimeStore,
    pub validities: ValidityStore,
    pub generation: GenerationId,
}

pub(crate) fn open_active_generation(
    storage: &Storage,
    root: &Path,
) -> Result<TemporalRuntimeParts> {
    let manager = GenerationManager::new(storage);
    if let Some(active) = manager.active(TARGET)? {
        return open_parts(root, active.id);
    }
    let id = GenerationId("bootstrap-v1".into());
    manager.create(&GenerationMetadata {
        schema_version: GENERATION_SCHEMA_VERSION,
        id: id.clone(),
        target: TARGET.into(),
        profile_name: "temporal-v1".into(),
        profile_version: 1,
        backend_schema: "rocksdb-temporal-v1".into(),
        source_snapshot_sequence: storage.latest_mutation_sequence()?,
        state: GenerationState::Draft,
        build_checkpoint: 0,
        document_count: 0,
        checksum: None,
        failure: None,
        created_at: Utc::now(),
        validated_at: None,
    })?;
    manager.start_build(TARGET, &id)?;
    let parts = open_parts(root, id.clone())?;
    manager.begin_validation(TARGET, &id)?;
    manager.validation_passed(TARGET, &id, 0, "empty-bootstrap")?;
    manager.activate(TARGET, &id)?;
    Ok(parts)
}

impl ZlfDatabase {
    pub(crate) fn catch_up_temporal(&self) -> Result<()> {
        let coordinator = IndexCoordinator::new(&self.storage, CoordinatorConfig::default());
        coordinator.register_target(TARGET)?;
        let target = TemporalIndexTarget::new(
            self.events.as_ref(),
            self.validities.as_ref(),
            self.temporal_generation.clone(),
        );
        loop {
            let enqueued = coordinator.enqueue_available(TARGET)?;
            while coordinator.process_next(TARGET, &target)? {}
            if enqueued == 0 {
                break;
            }
        }
        Ok(())
    }
}

fn open_parts(root: &Path, generation: GenerationId) -> Result<TemporalRuntimeParts> {
    let path = root.join("generations").join(&generation.0);
    Ok(TemporalRuntimeParts {
        events: EventTimeStore::open(path.join("events"))?,
        validities: ValidityStore::open(path.join("validities"))?,
        generation,
    })
}
