use std::path::Path;
use std::sync::{Arc, RwLock};

use zlf_core::{Edge, Node, Result, ZlfError};
use zlf_index::{BM25Index, TemporalEntry, TemporalIndex, VectorEntry, VectorIndex};
use zlf_prolog::wam::{
    CompiledRuleArtifact, CompositeFactProvider, IndexFactProvider, IndexedStorageFactWriter,
    IntrospectionProvider, PredicateRegistry, RuleDependencyGraph, StorageFactProvider,
    StorageRuleStore, WamRuntime,
};
mod helpers;
mod registry;
mod retract;

use zlf_prolog::{PrologParser, PrologRule, Query, Term};
use zlf_storage::Storage;

pub struct ZlfDatabase {
    storage: Arc<Storage>,
    temporal: Arc<TemporalIndex>,
    bm25: Arc<BM25Index>,
    vector: Arc<VectorIndex>,
    rules: RwLock<Vec<CompiledRuleArtifact>>,
    registry: RwLock<PredicateRegistry>,
}

impl ZlfDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let storage = Arc::new(Storage::open(path.join("storage"))?);
        Self::from_parts(
            Arc::clone(&storage),
            TemporalIndex::open(path.join("temporal"))?,
            BM25Index::open(path.join("bm25"))?,
            VectorIndex::open(path.join("vector"))?,
        )
    }

    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let storage = Arc::new(Storage::open_existing(path.join("storage"))?);
        Self::from_parts(
            Arc::clone(&storage),
            TemporalIndex::open(path.join("temporal"))?,
            BM25Index::open(path.join("bm25"))?,
            VectorIndex::open(path.join("vector"))?,
        )
    }

    fn from_parts(
        storage: Arc<Storage>,
        temporal: TemporalIndex,
        bm25: BM25Index,
        vector: VectorIndex,
    ) -> Result<Self> {
        let rules = StorageRuleStore::new(storage.as_ref())
            .all_rules()
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        let mut pred_registry = PredicateRegistry::new();
        registry::populate_registry(&storage, &rules, &mut pred_registry)?;
        Ok(Self {
            storage,
            temporal: Arc::new(temporal),
            bm25: Arc::new(bm25),
            vector: Arc::new(vector),
            rules: RwLock::new(rules),
            registry: RwLock::new(pred_registry),
        })
    }

    pub fn query_prolog(&self, source: &str) -> Result<Vec<serde_json::Value>> {
        match PrologParser::parse_query(source)? {
            Query::Goal(term) => self.execute_terms(&[term]),
            Query::Goals(terms) => self.execute_terms(&terms),
            Query::RuleDef(rule) => {
                self.store_rule(rule)?;
                Ok(Vec::new())
            }
        }
    }

    pub fn apply_fact(&self, fact: &Term) -> Result<()> {
        IndexedStorageFactWriter::new(self.storage.as_ref())
            .with_bm25(self.bm25.as_ref())
            .apply_fact(fact)
            .map_err(|e| ZlfError::Internal(e.to_string()))
    }

    pub fn store_rule(&self, rule: PrologRule) -> Result<()> {
        let artifact =
            CompiledRuleArtifact::compile(&rule).map_err(|e| ZlfError::Internal(e.to_string()))?;
        StorageRuleStore::new(self.storage.as_ref())
            .add_compiled_rule(&artifact)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        self.rules
            .write()
            .map_err(|e| ZlfError::Internal(e.to_string()))?
            .push(artifact);
        Ok(())
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
        self.index_node_text(&created)?;
        Ok(created)
    }

    pub fn get_node(&self, id: &str) -> Result<Option<Node>> {
        self.storage.get_node(id)
    }

    pub fn add_edge(&self, edge: Edge) -> Result<Edge> {
        self.storage.create_edge(edge)
    }

    pub fn get_edge(&self, id: &str) -> Result<Option<Edge>> {
        self.storage.get_edge(id)
    }

    pub fn get_all_nodes(&self) -> Result<Vec<Node>> {
        self.storage
            .scan_prefix("node:")?
            .into_iter()
            .map(|(_, value)| {
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))
            })
            .collect()
    }

    pub fn get_all_edges(&self) -> Result<Vec<Edge>> {
        self.storage
            .scan_prefix("edge:")?
            .into_iter()
            .map(|(_, value)| {
                bincode::deserialize(&value).map_err(|e| ZlfError::Serialization(e.to_string()))
            })
            .collect()
    }

    pub fn search(&self, query: &str) -> Result<Vec<(String, f32)>> {
        self.bm25.search(query)
    }

    pub fn index_text(&self, node_id: &str, text: &str) -> Result<()> {
        self.bm25.index_text(node_id, text)
    }

    pub fn index_embedding(&self, node_id: &str, embedding: &[f32], model: &str) -> Result<()> {
        self.vector.add_entry(VectorEntry {
            node_id: node_id.to_string(),
            embedding: embedding.to_vec(),
            model: model.to_string(),
        })
    }

    pub fn similar(
        &self,
        node_id: &str,
        threshold: f32,
        limit: usize,
    ) -> Result<Vec<(String, f32)>> {
        match self.vector.get_entry(node_id)? {
            Some(entry) => self.vector.find_similar(&entry.embedding, threshold, limit),
            None => Ok(Vec::new()),
        }
    }

    fn execute_terms(&self, terms: &[Term]) -> Result<Vec<serde_json::Value>> {
        let storage_provider = StorageFactProvider::new(self.storage.as_ref());
        let index_provider = IndexFactProvider::new()
            .with_bm25(self.bm25.as_ref())
            .with_vector(self.vector.as_ref())
            .with_temporal(self.temporal.as_ref());
        let reg = self.registry.read().map_err(lock_error)?;
        let dep_graph = RuleDependencyGraph::from_rules(&self.rules.read().map_err(lock_error)?);
        let introspection = IntrospectionProvider::new(reg.clone(), dep_graph);
        let provider = CompositeFactProvider::new()
            .with(&storage_provider)
            .with(&index_provider)
            .with(&introspection);
        let mut runtime = WamRuntime::new(64);
        for artifact in self.rules.read().map_err(lock_error)?.iter().cloned() {
            runtime.add_compiled_rule(artifact);
        }
        let (query, wrapper) = helpers::query_plan(terms);
        if let Some(rule) = wrapper {
            runtime.add_rule(rule);
        }
        let rows = runtime
            .query_all_with_provider(&query, &provider)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        Ok(self.dedupe_results(rows.into_iter().map(helpers::solution_to_json).collect()))
    }

    fn index_node_text(&self, node: &Node) -> Result<()> {
        let mut parts = vec![node.id.clone()];
        parts.extend(node.labels.iter().cloned());
        for value in node.properties.values() {
            helpers::collect_text(value, &mut parts);
        }
        self.bm25.index_text(&node.id, &parts.join(" "))
    }
}

fn lock_error(error: impl std::fmt::Display) -> ZlfError {
    helpers::lock_error(error)
}
