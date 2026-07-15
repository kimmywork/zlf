use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use zlf_core::{Result, ZlfError};
use zlf_index::{
    bge_m3_dense_v1, ranked_page, EmbeddingModelProfile, ExactVectorStore, GenerationId,
    GenerationMetadata, GenerationState, HnswVectorIndex, IndexPage, IndexPageRequest, VectorHit,
    VectorQuery, VectorRecord, VectorSearchBackend, GENERATION_SCHEMA_VERSION,
};
use zlf_storage::Storage;

use crate::{EmbeddingModelProfileStore, GenerationManager, VectorIndexStrategy};

const TARGET: &str = "vector";
const BACKEND_SCHEMA: &str = "rocksdb-exact-vector-v1";

pub(crate) struct VectorRuntimeParts {
    pub store: ExactVectorStore,
    pub generation: GenerationId,
    pub model: EmbeddingModelProfile,
}

pub(crate) struct VectorRuntime {
    pub store: Arc<ExactVectorStore>,
    pub generation: GenerationId,
    pub model: EmbeddingModelProfile,
    strategy: VectorIndexStrategy,
    hnsw_root: PathBuf,
    hnsw: Arc<RwLock<Option<Arc<HnswVectorIndex>>>>,
    rebuilding: Arc<AtomicBool>,
    rebuild_pending: Arc<AtomicBool>,
    stale: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexStatus {
    pub enabled: bool,
    pub strategy: String,
    pub ann_ready: bool,
    pub ann_rebuilding: bool,
    pub ann_stale: bool,
    pub exact_fallback: bool,
}

impl VectorRuntime {
    pub fn new(parts: VectorRuntimeParts, root: &Path, strategy: VectorIndexStrategy) -> Self {
        let hnsw_root = root.join("hnsw").join(&parts.generation.0);
        let hnsw = match strategy {
            VectorIndexStrategy::Hnsw(_) => HnswVectorIndex::open(&hnsw_root)
                .ok()
                .filter(|index| {
                    parts
                        .store
                        .records(&parts.generation.0, &parts.model.id, parts.model.version)
                        .is_ok_and(|records| index.matches_records(&records, &parts.model))
                })
                .map(Arc::new),
            _ => None,
        };
        Self {
            store: Arc::new(parts.store),
            generation: parts.generation,
            model: parts.model,
            strategy,
            hnsw_root,
            hnsw: Arc::new(RwLock::new(hnsw)),
            rebuilding: Arc::new(AtomicBool::new(false)),
            rebuild_pending: Arc::new(AtomicBool::new(false)),
            stale: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn status(&self) -> VectorIndexStatus {
        let ready = self.hnsw.read().is_ok_and(|value| value.is_some());
        VectorIndexStatus {
            enabled: true,
            strategy: match self.strategy {
                VectorIndexStrategy::Exact => "exact",
                VectorIndexStrategy::Hnsw(_) => "hnsw",
                VectorIndexStrategy::Disabled => "disabled",
            }
            .into(),
            ann_ready: ready,
            ann_rebuilding: self.rebuilding.load(Ordering::Acquire),
            ann_stale: self.stale.load(Ordering::Acquire),
            exact_fallback: matches!(self.strategy, VectorIndexStrategy::Hnsw(_))
                && (!ready || self.stale.load(Ordering::Acquire)),
        }
    }

    pub fn mark_stale(&self) {
        if matches!(self.strategy, VectorIndexStrategy::Hnsw(_)) {
            self.stale.store(true, Ordering::Release);
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn request_rebuild(&self) -> Result<bool> {
        let VectorIndexStrategy::Hnsw(options) = self.strategy else {
            return Err(ZlfError::IndexUnavailable {
                index: "hnsw".into(),
                operation: "request_rebuild".into(),
            });
        };
        if self
            .rebuilding
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            self.rebuild_pending.store(true, Ordering::Release);
            return Ok(false);
        }
        let store = Arc::clone(&self.store);
        let generation = self.generation.clone();
        let model = self.model.clone();
        let root = self.hnsw_root.clone();
        let target = Arc::clone(&self.hnsw);
        let rebuilding = Arc::clone(&self.rebuilding);
        let pending = Arc::clone(&self.rebuild_pending);
        let stale = Arc::clone(&self.stale);
        std::thread::spawn(move || loop {
            let rebuilt = store
                .records(&generation.0, &model.id, model.version)
                .and_then(|records| {
                    HnswVectorIndex::build_and_publish(&root, records, &model, options)
                });
            if let Ok(index) = rebuilt {
                if let Ok(mut active) = target.write() {
                    *active = Some(Arc::new(index));
                    stale.store(false, Ordering::Release);
                }
            }
            if pending.swap(false, Ordering::AcqRel) {
                continue;
            }
            rebuilding.store(false, Ordering::Release);
            if pending.swap(false, Ordering::AcqRel)
                && rebuilding
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            {
                continue;
            }
            break;
        });
        Ok(true)
    }

    pub fn search_page(
        &self,
        query: &VectorQuery,
        page: IndexPageRequest,
    ) -> Result<IndexPage<VectorHit>> {
        let mut request = query.clone();
        request.top_k = page.probe_limit();
        let hits = self.search(&request, &self.model)?;
        ranked_page(hits, page).map_err(|error| ZlfError::Internal(error.to_string()))
    }
}

impl VectorSearchBackend for VectorRuntime {
    fn search(
        &self,
        query: &VectorQuery,
        profile: &EmbeddingModelProfile,
    ) -> Result<Vec<VectorHit>> {
        if matches!(self.strategy, VectorIndexStrategy::Hnsw(_))
            && !self.stale.load(Ordering::Acquire)
        {
            if let Ok(active) = self.hnsw.read() {
                if let Some(index) = active.as_ref() {
                    if let Ok(hits) = index.search(query, profile) {
                        return Ok(hits);
                    }
                }
            }
        }
        self.store.search(query, profile)
    }

    fn records_for_entity(
        &self,
        generation: &str,
        model_profile: &str,
        model_version: u32,
        entity: &zlf_core::EntityRef,
    ) -> Result<Vec<VectorRecord>> {
        self.store
            .records_for_entity(generation, model_profile, model_version, entity)
    }
}

pub(crate) fn open_active_generation(storage: &Storage, root: &Path) -> Result<VectorRuntimeParts> {
    let profile = bge_m3_dense_v1();
    EmbeddingModelProfileStore::new(storage).put(&profile)?;
    let manager = GenerationManager::new(storage);
    if let Some(active) = manager.active(TARGET)? {
        let store = ExactVectorStore::open(generation_path(root, &active.id))?;
        return Ok(VectorRuntimeParts {
            store,
            generation: active.id,
            model: profile,
        });
    }
    bootstrap_generation(storage, root, profile)
}

fn bootstrap_generation(
    storage: &Storage,
    root: &Path,
    profile: EmbeddingModelProfile,
) -> Result<VectorRuntimeParts> {
    let manager = GenerationManager::new(storage);
    let id = GenerationId("bootstrap-v1".into());
    let metadata = GenerationMetadata {
        schema_version: GENERATION_SCHEMA_VERSION,
        id: id.clone(),
        target: TARGET.into(),
        profile_name: profile.id.clone(),
        profile_version: profile.version,
        backend_schema: BACKEND_SCHEMA.into(),
        source_snapshot_sequence: storage.latest_mutation_sequence()?,
        state: GenerationState::Draft,
        build_checkpoint: 0,
        document_count: 0,
        checksum: None,
        failure: None,
        created_at: Utc::now(),
        validated_at: None,
    };
    manager.create(&metadata)?;
    manager.start_build(TARGET, &id)?;
    let store = ExactVectorStore::open(generation_path(root, &id))?;
    manager.begin_validation(TARGET, &id)?;
    manager.validation_passed(TARGET, &id, 0, "empty-bootstrap")?;
    manager.activate(TARGET, &id)?;
    Ok(VectorRuntimeParts {
        store,
        generation: id,
        model: profile,
    })
}

pub(crate) fn disabled_status() -> VectorIndexStatus {
    VectorIndexStatus {
        enabled: false,
        strategy: "disabled".into(),
        ann_ready: false,
        ann_rebuilding: false,
        ann_stale: false,
        exact_fallback: false,
    }
}

fn generation_path(root: &Path, id: &GenerationId) -> PathBuf {
    root.join("generations").join(&id.0)
}
