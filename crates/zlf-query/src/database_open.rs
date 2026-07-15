use std::path::Path;
use std::sync::{Arc, RwLock};

use zlf_core::{Result, ZlfError};
use zlf_index::{BM25Index, GenerationId};
use zlf_prolog::wam::{PredicateRegistry, RocksTableBackend, StorageRuleStore, TableManager};
use zlf_storage::Storage;

use crate::{
    bm25_runtime, registry, table, temporal_runtime, vector_runtime, CoordinatorConfig,
    IndexCoordinator, ZlfDatabase, ZlfDatabaseOptions,
};

impl ZlfDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_options(path, ZlfDatabaseOptions::default())
    }

    pub fn open_with_options(path: impl AsRef<Path>, options: ZlfDatabaseOptions) -> Result<Self> {
        Self::open_configured(path.as_ref(), options, false)
    }

    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_existing_with_options(path, ZlfDatabaseOptions::default())
    }

    pub fn open_existing_with_options(
        path: impl AsRef<Path>,
        options: ZlfDatabaseOptions,
    ) -> Result<Self> {
        Self::open_configured(path.as_ref(), options, true)
    }

    fn open_configured(path: &Path, options: ZlfDatabaseOptions, existing: bool) -> Result<Self> {
        let storage = Arc::new(if existing {
            Storage::open_existing(path.join("storage"))?
        } else {
            Storage::open(path.join("storage"))?
        });
        let bm25_root = path.join("bm25");
        let (bm25, generation) = bm25_runtime::open_active_generation(&storage, &bm25_root)?;
        let vector_root = path.join("vector");
        let vector = if options.vector_index.is_enabled() {
            let parts = vector_runtime::open_active_generation(&storage, &vector_root)?;
            Some(vector_runtime::VectorRuntime::new(
                parts,
                &vector_root,
                options.vector_index,
            ))
        } else {
            IndexCoordinator::new(&storage, CoordinatorConfig::default())
                .disable_target("vector")?;
            None
        };
        let temporal = temporal_runtime::open_active_generation(&storage, &path.join("temporal"))?;
        Self::from_parts(storage, bm25, bm25_root, generation, vector, temporal)
    }

    fn from_parts(
        storage: Arc<Storage>,
        bm25: BM25Index,
        bm25_root: std::path::PathBuf,
        bm25_generation: GenerationId,
        vector: Option<vector_runtime::VectorRuntime>,
        temporal: temporal_runtime::TemporalRuntimeParts,
    ) -> Result<Self> {
        let rules = StorageRuleStore::new(storage.as_ref())
            .all_rules()
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        let mut registry_value = PredicateRegistry::new();
        registry::populate_registry(&storage, &rules, &mut registry_value)?;
        let table_manager = Arc::new(TableManager::with_backend(Arc::new(
            RocksTableBackend::new(Arc::clone(&storage)),
        )));
        let database = Self {
            tabled: RwLock::new(table::load_declarations(&storage)?),
            storage,
            events: Arc::new(temporal.events),
            validities: Arc::new(temporal.validities),
            temporal_generation: temporal.generation,
            bm25: RwLock::new(Arc::new(bm25)),
            bm25_root,
            bm25_generation: RwLock::new(bm25_generation),
            vector,
            rules: RwLock::new(rules),
            registry: RwLock::new(registry_value),
            table_manager,
            prepared_retrievals: crate::retrieval_preparation::PreparedRetrievalRegistry::new(),
        };
        database.catch_up_indexes()?;
        Ok(database)
    }
}
