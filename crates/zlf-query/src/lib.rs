use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use zlf_core::{Edge, Node, Result, ZlfError};
use zlf_index::{BM25Index, EventTimeStore, GenerationId, ValidityStore};
use zlf_prolog::wam::{
    CompiledRuleArtifact, CompositeFactProvider, GraphAlgorithmProvider, GraphViewProvider,
    IndexFactProvider, IntrospectionProvider, PredicateRegistry, StorageFactProvider,
    StorageFactWriter, StorageRuleStore, TableManager, WamRuntime,
};
mod bm25_runtime;
mod bm25_target;
mod coordinator;
mod coordinator_store;
mod database_open;
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
mod retrieval_candidates;
mod retrieval_execution;
mod retrieval_preparation;
mod retrieval_provider;
mod table;
mod temporal_manifest_store;
mod temporal_projection;
mod temporal_runtime;
mod temporal_target;
mod vector_embedding_target;
mod vector_facade;
mod vector_runtime;
mod vector_strategy;

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
pub use retrieval_execution::{RetrievalExecutionMetadata, RetrievalExecutionResult};
pub use retrieval_preparation::{
    PreparedIndexSnapshot, PreparedRetrieval, PreparedRetrievalHandle, RetrievalPreparationError,
};
pub use temporal_target::TemporalIndexTarget;
pub use vector_embedding_target::VectorEmbeddingTarget;
pub use vector_runtime::VectorIndexStatus;
pub use vector_strategy::{VectorIndexStrategy, ZlfDatabaseOptions};

use helpers::lock_error;
use zlf_prolog::{PrologParser, PrologRule, Query, Term};
use zlf_storage::Storage;

fn contains_vector_predicate(term: &Term) -> bool {
    match term {
        Term::Compound { name, args } => {
            name == "vector_similar" || args.iter().any(contains_vector_predicate)
        }
        Term::List(items) => items.iter().any(contains_vector_predicate),
        Term::Object(fields) => fields
            .iter()
            .any(|(_, value)| contains_vector_predicate(value)),
        _ => false,
    }
}

pub struct ZlfDatabase {
    storage: Arc<Storage>,
    events: Arc<EventTimeStore>,
    validities: Arc<ValidityStore>,
    temporal_generation: GenerationId,
    bm25: RwLock<Arc<BM25Index>>,
    bm25_root: PathBuf,
    bm25_generation: RwLock<GenerationId>,
    vector: Option<vector_runtime::VectorRuntime>,
    rules: RwLock<Vec<CompiledRuleArtifact>>,
    registry: RwLock<PredicateRegistry>,
    tabled: RwLock<HashSet<zlf_prolog::wam::PredicateKey>>,
    table_manager: Arc<TableManager>,
    prepared_retrievals: retrieval_preparation::PreparedRetrievalRegistry,
}

impl ZlfDatabase {
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

    pub fn add_node(&self, node: Node) -> Result<Node> {
        let created = self.storage.create_node(node)?;
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

    pub(crate) fn require_vector(&self, operation: &str) -> Result<&vector_runtime::VectorRuntime> {
        self.vector
            .as_ref()
            .ok_or_else(|| ZlfError::IndexUnavailable {
                index: "vector_embedding".into(),
                operation: operation.into(),
            })
    }

    #[allow(clippy::too_many_lines)]
    fn execute_terms(&self, terms: &[Term]) -> Result<Vec<serde_json::Value>> {
        if self.vector.is_none() && terms.iter().any(contains_vector_predicate) {
            return Err(ZlfError::IndexUnavailable {
                index: "vector_embedding".into(),
                operation: "prolog_vector_query".into(),
            });
        }
        let storage_provider = StorageFactProvider::new(self.storage.as_ref());
        let retrieval_provider = retrieval_provider::PreparedRetrievalProvider::new(self);
        let bm25 = self.bm25.read().map_err(lock_error)?.clone();
        let mut index_provider = IndexFactProvider::new().with_bm25(bm25.as_ref());
        if let Some(vector) = self.vector.as_ref() {
            index_provider =
                index_provider.with_vector_backend(vector, &vector.model, &vector.generation);
        }
        let index_provider = index_provider.with_temporal(
            self.events.as_ref(),
            self.validities.as_ref(),
            &self.temporal_generation,
        );
        let reg = self.registry.read().map_err(lock_error)?.clone();
        let rules = self.rules.read().map_err(lock_error)?.clone();
        let introspection = IntrospectionProvider::new(reg, &rules);
        let graph_view = GraphViewProvider::new(self.storage.as_ref());
        let graph_algo = GraphAlgorithmProvider::new(self.storage.as_ref());
        let provider = CompositeFactProvider::new()
            .with(&storage_provider)
            .with(&index_provider)
            .with(&retrieval_provider)
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
