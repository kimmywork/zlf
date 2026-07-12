use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use zlf_core::{Result, ZlfError};
use zlf_index::{
    BM25DocumentHit, BM25Index, GenerationId, GenerationMetadata, GenerationState,
    GENERATION_SCHEMA_VERSION,
};
use zlf_storage::Storage;

use crate::{
    helpers::lock_error, Bm25IndexTarget, CoordinatorConfig, GenerationManager, IndexCoordinator,
    IndexProfileStore, ZlfDatabase,
};

const TARGET: &str = "bm25";
const BACKEND_SCHEMA: &str = "tantivy-bm25-v1";

pub(crate) fn open_active_generation(
    storage: &Storage,
    root: &Path,
) -> Result<(BM25Index, GenerationId)> {
    let manager = GenerationManager::new(storage);
    if let Some(active) = manager.active(TARGET)? {
        let index = BM25Index::open(generation_path(root, &active.id))?;
        return Ok((index, active.id));
    }
    let id = GenerationId("bootstrap-v1".into());
    let metadata = metadata(storage, id.clone(), "_bootstrap", 0)?;
    manager.create(&metadata)?;
    manager.start_build(TARGET, &id)?;
    let index = BM25Index::open(generation_path(root, &id))?;
    manager.begin_validation(TARGET, &id)?;
    manager.validation_passed(TARGET, &id, 0, "empty-bootstrap")?;
    manager.activate(TARGET, &id)?;
    Ok((index, id))
}

impl ZlfDatabase {
    pub fn search_bm25(
        &self,
        query: &str,
        top_k: usize,
        fields: &[String],
        explain: bool,
    ) -> Result<Vec<BM25DocumentHit>> {
        let weights = self.active_bm25_weights()?;
        self.bm25
            .read()
            .map_err(lock_error)?
            .search_document_top_k(query, top_k, fields, &weights, explain)
    }

    pub fn rebuild_bm25_generation(&self) -> Result<GenerationId> {
        let (profile_name, profile_version) = self.active_profile_identity()?;
        let id = GenerationId(format!(
            "g-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let manager = GenerationManager::new(&self.storage);
        manager.create(&metadata(
            &self.storage,
            id.clone(),
            &profile_name,
            profile_version,
        )?)?;
        manager.start_build(TARGET, &id)?;
        match self.build_and_activate(&manager, &id) {
            Ok(()) => Ok(id),
            Err(error) => {
                let generation = manager
                    .get(TARGET, &id)?
                    .ok_or_else(|| ZlfError::Internal("generation disappeared".into()))?;
                if generation.state != GenerationState::Active {
                    manager.fail(TARGET, &id, &error.to_string())?;
                }
                Err(error)
            }
        }
    }

    pub(crate) fn catch_up_bm25(&self) -> Result<()> {
        let coordinator = IndexCoordinator::new(&self.storage, CoordinatorConfig::default());
        coordinator.register_target(TARGET)?;
        let generation = self.bm25_generation.read().map_err(lock_error)?.clone();
        let index = self.bm25.read().map_err(lock_error)?.clone();
        let target = Bm25IndexTarget::new(index.as_ref(), &manifest_scope(&generation));
        loop {
            let enqueued = coordinator.enqueue_available(TARGET)?;
            while coordinator.process_next(TARGET, &target)? {}
            if enqueued == 0 {
                break;
            }
        }
        let progress = coordinator.progress(TARGET)?;
        if progress.published_watermark < progress.scanned_watermark {
            return Err(ZlfError::Internal(format!(
                "BM25 indexing stopped at watermark {} of {}",
                progress.published_watermark, progress.scanned_watermark
            )));
        }
        Ok(())
    }

    fn build_and_activate(&self, manager: &GenerationManager<'_>, id: &GenerationId) -> Result<()> {
        let index = Arc::new(BM25Index::open(generation_path(&self.bm25_root, id))?);
        Bm25IndexTarget::new(index.as_ref(), &manifest_scope(id)).rebuild(&self.storage)?;
        manager.checkpoint(TARGET, id, self.storage.latest_mutation_sequence()?)?;
        manager.begin_validation(TARGET, id)?;
        let count = index.document_count();
        manager.validation_passed(TARGET, id, count, &format!("{BACKEND_SCHEMA}:{count}"))?;
        manager.activate(TARGET, id)?;
        *self.bm25.write().map_err(lock_error)? = index;
        *self.bm25_generation.write().map_err(lock_error)? = id.clone();
        self.catch_up_bm25()
    }

    fn active_profile_identity(&self) -> Result<(String, u32)> {
        let store = IndexProfileStore::new(&self.storage);
        for name in store
            .list()?
            .into_iter()
            .map(|profile| profile.name)
            .collect::<BTreeSet<_>>()
        {
            if let Some(profile) = store.active(&name)? {
                return Ok((profile.name, profile.version));
            }
        }
        Ok(("_none".into(), 0))
    }

    fn active_bm25_weights(&self) -> Result<BTreeMap<String, f32>> {
        let store = IndexProfileStore::new(&self.storage);
        let names = store
            .list()?
            .into_iter()
            .map(|profile| profile.name)
            .collect::<BTreeSet<_>>();
        let mut weights = BTreeMap::new();
        for name in names {
            if let Some(profile) = store.active(&name)? {
                for (field, options) in profile.fields {
                    if let Some(bm25) = options.bm25 {
                        weights.insert(field, bm25.weight);
                    }
                }
            }
        }
        Ok(weights)
    }
}

fn metadata(
    storage: &Storage,
    id: GenerationId,
    profile_name: &str,
    profile_version: u32,
) -> Result<GenerationMetadata> {
    Ok(GenerationMetadata {
        schema_version: GENERATION_SCHEMA_VERSION,
        id,
        target: TARGET.into(),
        profile_name: profile_name.into(),
        profile_version,
        backend_schema: BACKEND_SCHEMA.into(),
        source_snapshot_sequence: storage.latest_mutation_sequence()?,
        state: GenerationState::Draft,
        build_checkpoint: 0,
        document_count: 0,
        checksum: None,
        failure: None,
        created_at: Utc::now(),
        validated_at: None,
    })
}

fn generation_path(root: &Path, id: &GenerationId) -> std::path::PathBuf {
    root.join("generations").join(&id.0)
}

fn manifest_scope(id: &GenerationId) -> String {
    format!("{TARGET}:{}", id.0)
}
