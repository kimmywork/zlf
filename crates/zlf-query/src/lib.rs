use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use zlf_core::{Edge, Node, Result, ZlfError};
use zlf_index::{
    BM25Index, EmbeddingModelProfile, ExactVectorStore, GenerationId, TemporalEntry, TemporalIndex,
};
use zlf_prolog::wam::{
    CompiledRuleArtifact, CompositeFactProvider, GraphAlgorithmProvider, GraphViewProvider,
    IndexFactProvider, IntrospectionProvider, PredicateRegistry, RocksTableBackend,
    StorageFactProvider, StorageFactWriter, StorageRuleStore, TableManager, WamRuntime,
};
mod bm25_runtime;
mod bm25_target;
mod coordinator;
mod coordinator_store;
mod embedding_job_store;
mod embedding_worker_v2;
mod explain;
mod fake_documents;
mod fake_index_target;
mod generation_facade;
mod generation_manager;
mod generation_rollback;
mod graph_facade;
mod helpers;
mod index_facade;
mod index_wait;
mod manifest_store;
mod model_profile_store;
mod mutation;
mod profile_store;
mod proof;
mod registry;
mod table;
mod vector_embedding_target;
mod vector_runtime;

pub use bm25_target::Bm25IndexTarget;
pub use coordinator::{
    CoordinatorConfig, DurableIndexJob, IndexCoordinator, IndexJobState, IndexTarget,
    TargetApplyError, TargetProgress,
};
pub use embedding_job_store::EmbeddingJobStore;
pub use embedding_worker_v2::{
    BatchEmbeddingProvider, DurableEmbeddingWorker, EmbeddingProviderFailure,
};
pub use explain::{AccessPath, ArgumentMode, PlannedGoal, QueryPlan};
pub use fake_index_target::{FakeFailureMode, FakeIndexTarget};
pub use generation_manager::GenerationManager;
pub use index_wait::wait_for_indexes;
pub use manifest_store::IndexManifestStore;
pub use model_profile_store::EmbeddingModelProfileStore;
pub use profile_store::IndexProfileStore;
pub use vector_embedding_target::VectorEmbeddingTarget;

use helpers::lock_error;
use zlf_prolog::{PrologParser, PrologRule, Query, Term};
use zlf_storage::Storage;

pub struct ZlfDatabase {
    storage: Arc<Storage>,
    temporal: Arc<TemporalIndex>,
    bm25: RwLock<Arc<BM25Index>>,
    bm25_root: PathBuf,
    bm25_generation: RwLock<GenerationId>,
    vector: Arc<ExactVectorStore>,
    vector_generation: GenerationId,
    vector_model: EmbeddingModelProfile,
    rules: RwLock<Vec<CompiledRuleArtifact>>,
    registry: RwLock<PredicateRegistry>,
    tabled: RwLock<HashSet<zlf_prolog::wam::PredicateKey>>,
    table_manager: Arc<TableManager>,
}

impl ZlfDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let storage = Arc::new(Storage::open(path.join("storage"))?);
        let bm25_root = path.join("bm25");
        let (bm25, generation) = bm25_runtime::open_active_generation(&storage, &bm25_root)?;
        let vector = vector_runtime::open_active_generation(&storage, &path.join("vector"))?;
        Self::from_parts(
            Arc::clone(&storage),
            TemporalIndex::open(path.join("temporal"))?,
            bm25,
            bm25_root,
            generation,
            vector,
        )
    }

    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let storage = Arc::new(Storage::open_existing(path.join("storage"))?);
        let bm25_root = path.join("bm25");
        let (bm25, generation) = bm25_runtime::open_active_generation(&storage, &bm25_root)?;
        let vector = vector_runtime::open_active_generation(&storage, &path.join("vector"))?;
        Self::from_parts(
            Arc::clone(&storage),
            TemporalIndex::open(path.join("temporal"))?,
            bm25,
            bm25_root,
            generation,
            vector,
        )
    }

    fn from_parts(
        storage: Arc<Storage>,
        temporal: TemporalIndex,
        bm25: BM25Index,
        bm25_root: PathBuf,
        bm25_generation: GenerationId,
        vector: vector_runtime::VectorRuntimeParts,
    ) -> Result<Self> {
        let rules = StorageRuleStore::new(storage.as_ref())
            .all_rules()
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        let mut pred_registry = PredicateRegistry::new();
        registry::populate_registry(&storage, &rules, &mut pred_registry)?;
        let table_manager = Arc::new(TableManager::with_backend(Arc::new(
            RocksTableBackend::new(Arc::clone(&storage)),
        )));
        let tabled = table::load_declarations(&storage)?;
        let database = Self {
            storage,
            temporal: Arc::new(temporal),
            bm25: RwLock::new(Arc::new(bm25)),
            bm25_root,
            bm25_generation: RwLock::new(bm25_generation),
            vector: Arc::new(vector.store),
            vector_generation: vector.generation,
            vector_model: vector.model,
            rules: RwLock::new(rules),
            registry: RwLock::new(pred_registry),
            tabled: RwLock::new(tabled),
            table_manager,
        };
        database.catch_up_indexes()?;
        Ok(database)
    }

    pub fn query_prolog(&self, source: &str) -> Result<Vec<serde_json::Value>> {
        match PrologParser::parse_query(source)? {
            Query::Goal(term) => self.execute_terms(&[term]),
            Query::Goals(terms) => self.execute_terms(&terms),
            Query::RuleDef(rule) => {
                self.store_rule(rule)?;
                Ok(Vec::new())
            }
            Query::Directive(directive) => {
                self.apply_directive(&directive)?;
                Ok(Vec::new())
            }
        }
    }

    pub fn apply_fact(&self, fact: &Term) -> Result<()> {
        StorageFactWriter::new(self.storage.as_ref())
            .apply_fact(fact)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        self.invalidate_fact(fact)?;
        self.refresh_registry()?;
        self.catch_up_indexes()
    }

    pub fn store_rule(&self, rule: PrologRule) -> Result<()> {
        let artifact =
            CompiledRuleArtifact::compile(&rule).map_err(|e| ZlfError::Internal(e.to_string()))?;
        StorageRuleStore::new(self.storage.as_ref())
            .add_compiled_rule(&artifact)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        let predicate = artifact.key.clone();
        self.rules
            .write()
            .map_err(|e| ZlfError::Internal(e.to_string()))?
            .push(artifact);
        self.invalidate_predicates(&[predicate])?;
        self.refresh_registry()
    }

    pub fn table_metrics(&self) -> zlf_prolog::wam::TableMetricsSnapshot {
        self.table_manager.metrics()
    }

    pub fn get_rules(&self) -> Result<Vec<PrologRule>> {
        Ok(self
            .rules
            .read()
            .map_err(|e| ZlfError::Internal(e.to_string()))?
            .iter()
            .map(|artifact| artifact.source.clone())
            .collect())
    }

    pub fn add_node(&self, node: Node) -> Result<Node> {
        let created = self.storage.create_node(node)?;
        self.temporal.add_entry(TemporalEntry {
            node_id: created.id.clone(),
            valid_from: created.created_at,
            valid_to: None,
        })?;
        self.catch_up_indexes()?;
        self.invalidate_node(&created)?;
        Ok(created)
    }

    pub fn get_node(&self, id: &str) -> Result<Option<Node>> {
        self.storage.get_node(id)
    }

    pub fn add_edge(&self, edge: Edge) -> Result<Edge> {
        let edge = self.storage.create_edge(edge)?;
        self.catch_up_indexes()?;
        self.invalidate_edge(&edge.edge_type)?;
        Ok(edge)
    }

    pub fn get_edge(&self, id: &str) -> Result<Option<Edge>> {
        self.storage.get_edge(id)
    }

    #[allow(clippy::too_many_lines)]
    fn execute_terms(&self, terms: &[Term]) -> Result<Vec<serde_json::Value>> {
        let storage_provider = StorageFactProvider::new(self.storage.as_ref());
        let bm25 = self.bm25.read().map_err(lock_error)?.clone();
        let index_provider = IndexFactProvider::new()
            .with_bm25(bm25.as_ref())
            .with_exact_vector(
                self.vector.as_ref(),
                &self.vector_model,
                &self.vector_generation,
            )
            .with_temporal(self.temporal.as_ref());
        let reg = self.registry.read().map_err(lock_error)?.clone();
        let rules = self.rules.read().map_err(lock_error)?.clone();
        let introspection = IntrospectionProvider::new(reg, &rules);
        let graph_view = GraphViewProvider::new(self.storage.as_ref());
        let graph_algo = GraphAlgorithmProvider::new(self.storage.as_ref());
        let provider = CompositeFactProvider::new()
            .with(&storage_provider)
            .with(&index_provider)
            .with(&introspection)
            .with(&graph_view)
            .with(&graph_algo);
        let mut runtime = WamRuntime::new(64);
        runtime.set_table_manager(Arc::clone(&self.table_manager));
        for key in self.tabled.read().map_err(lock_error)?.iter().cloned() {
            runtime.declare_tabled(key);
        }
        for artifact in rules.iter().cloned() {
            runtime.add_compiled_rule(artifact);
        }
        let (query, wrapper) = helpers::query_plan(terms);
        if let Some(rule) = wrapper {
            runtime.add_rule(rule);
        }
        let rows = runtime
            .query_all_with_provider_and_storage(&query, &provider, self.storage.as_ref())
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        if terms.iter().any(table::contains_mutation) {
            self.refresh_after_mutation(terms)?;
            self.catch_up_indexes()?;
        }
        Ok(helpers::dedupe_results(
            rows.into_iter().map(helpers::solution_to_json).collect(),
        ))
    }

    fn reload_rules(&self) -> Result<()> {
        let rules = StorageRuleStore::new(self.storage.as_ref())
            .all_rules()
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        *self.rules.write().map_err(lock_error)? = rules;
        Ok(())
    }

    fn refresh_registry(&self) -> Result<()> {
        let rules = self.rules.read().map_err(lock_error)?;
        let mut registry = PredicateRegistry::new();
        registry::populate_registry(&self.storage, &rules, &mut registry)?;
        *self.registry.write().map_err(lock_error)? = registry;
        Ok(())
    }
}
