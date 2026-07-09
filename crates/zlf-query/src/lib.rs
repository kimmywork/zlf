use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use zlf_core::{Edge, Node, Result, Value, ZlfError};
use zlf_index::{BM25Index, TemporalEntry, TemporalIndex, VectorEntry, VectorIndex};
use zlf_prolog::wam::{
    CompiledRuleArtifact, CompositeFactProvider, IndexFactProvider, IndexedStorageFactWriter,
    StorageFactProvider, StorageRuleStore, WamRuntime,
};
use zlf_prolog::{PrologParser, PrologRule, Query, Term};
use zlf_storage::Storage;

pub struct ZlfDatabase {
    storage: Arc<Storage>,
    temporal: Arc<TemporalIndex>,
    bm25: Arc<BM25Index>,
    vector: Arc<VectorIndex>,
    rules: RwLock<Vec<CompiledRuleArtifact>>,
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
        Ok(Self {
            storage,
            temporal: Arc::new(temporal),
            bm25: Arc::new(bm25),
            vector: Arc::new(vector),
            rules: RwLock::new(rules),
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
        let provider = CompositeFactProvider::new()
            .with(&storage_provider)
            .with(&index_provider);
        let mut runtime = WamRuntime::new(64);
        for artifact in self.rules.read().map_err(lock_error)?.iter().cloned() {
            runtime.add_compiled_rule(artifact);
        }
        let (query, wrapper) = query_plan(terms)?;
        if let Some(rule) = wrapper {
            runtime.add_compiled_rule(
                CompiledRuleArtifact::compile(&rule)
                    .map_err(|e| ZlfError::Internal(e.to_string()))?,
            );
        }
        let rows = runtime
            .query_all_with_provider(&query, &provider)
            .map_err(|e| ZlfError::Internal(e.to_string()))?;
        Ok(rows.into_iter().map(solution_to_json).collect())
    }

    fn index_node_text(&self, node: &Node) -> Result<()> {
        let mut parts = vec![node.id.clone()];
        parts.extend(node.labels.iter().cloned());
        for value in node.properties.values() {
            collect_text(value, &mut parts);
        }
        self.bm25.index_text(&node.id, &parts.join(" "))
    }
}

fn query_plan(terms: &[Term]) -> Result<(Term, Option<PrologRule>)> {
    if terms.len() == 1 {
        return Ok((terms[0].clone(), None));
    }
    let vars = query_variables(terms);
    let head = Term::Compound {
        name: "__query".to_string(),
        args: vars
            .iter()
            .map(|name| Term::Variable(name.clone()))
            .collect(),
    };
    Ok((
        head.clone(),
        Some(PrologRule {
            head,
            body: terms.to_vec(),
        }),
    ))
}

fn query_variables(terms: &[Term]) -> Vec<String> {
    let mut vars = Vec::new();
    for term in terms {
        collect_variables(term, &mut vars);
    }
    vars
}

fn collect_variables(term: &Term, vars: &mut Vec<String>) {
    match term {
        Term::Variable(name) if name != "_" && !vars.contains(name) => vars.push(name.clone()),
        Term::Compound { args, .. } | Term::List(args) => {
            for arg in args {
                collect_variables(arg, vars);
            }
        }
        Term::Object(entries) => {
            for (_, value) in entries {
                collect_variables(value, vars);
            }
        }
        _ => {}
    }
}

fn solution_to_json(solution: HashMap<String, Term>) -> serde_json::Value {
    serde_json::Value::Object(
        solution
            .into_iter()
            .filter(|(name, _)| name != "_")
            .map(|(name, term)| (name, term_to_json(&term)))
            .collect(),
    )
}

fn term_to_json(term: &Term) -> serde_json::Value {
    match term {
        Term::Variable(name) => serde_json::json!({ "variable": name }),
        Term::Atom(name) | Term::String(name) => serde_json::json!(name),
        Term::Number(number) => serde_json::json!(number),
        Term::Compound { name, args } => serde_json::json!({
            "name": name,
            "args": args.iter().map(term_to_json).collect::<Vec<_>>()
        }),
        Term::List(items) => serde_json::json!(items.iter().map(term_to_json).collect::<Vec<_>>()),
        Term::Object(entries) => serde_json::Value::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), term_to_json(value)))
                .collect(),
        ),
    }
}

fn collect_text(value: &Value, parts: &mut Vec<String>) {
    match value {
        Value::String(text) => parts.push(text.clone()),
        Value::Number(number) => parts.push(number.to_string()),
        Value::Array(items) => items.iter().for_each(|item| collect_text(item, parts)),
        Value::Object(map) => map.values().for_each(|item| collect_text(item, parts)),
        _ => {}
    }
}

fn lock_error(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}
