use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::storage_writer::StorageFactWriter;
use zlf_index::{BM25Index, VectorEntry, VectorIndex};
use zlf_storage::Storage;

pub trait Embedder {
    fn model(&self) -> &str;
    fn embed(&self, text: &str) -> WamResult<Vec<f32>>;
}

pub struct IndexedStorageFactWriter<'a> {
    storage_writer: StorageFactWriter<'a>,
    bm25: Option<&'a BM25Index>,
    vector: Option<&'a VectorIndex>,
    embedder: Option<&'a dyn Embedder>,
}

impl<'a> IndexedStorageFactWriter<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self {
            storage_writer: StorageFactWriter::new(storage),
            bm25: None,
            vector: None,
            embedder: None,
        }
    }

    pub fn with_bm25(mut self, bm25: &'a BM25Index) -> Self {
        self.bm25 = Some(bm25);
        self
    }

    pub fn with_embedding(mut self, vector: &'a VectorIndex, embedder: &'a dyn Embedder) -> Self {
        self.vector = Some(vector);
        self.embedder = Some(embedder);
        self
    }

    pub fn apply_fact(&self, fact: &Term) -> WamResult<()> {
        self.storage_writer.apply_fact(fact)?;
        index_fact_text(fact, self.bm25, self.vector, self.embedder)
    }
}

fn index_fact_text(
    fact: &Term,
    bm25: Option<&BM25Index>,
    vector: Option<&VectorIndex>,
    embedder: Option<&dyn Embedder>,
) -> WamResult<()> {
    let Some((name, args)) = compound(fact) else {
        return Ok(());
    };
    match (name, args) {
        ("node", [id, props]) => index_object_text(atom(id)?, props, bm25, vector, embedder),
        ("node", [id, _, props]) => index_object_text(atom(id)?, props, bm25, vector, embedder),
        ("property", [id, _, value]) => index_value_text(atom(id)?, value, bm25, vector, embedder),
        (name, [id, value]) if name.starts_with("prop_") => {
            index_value_text(atom(id)?, value, bm25, vector, embedder)
        }
        _ => Ok(()),
    }
}

fn index_object_text(
    node_id: &str,
    term: &Term,
    bm25: Option<&BM25Index>,
    vector: Option<&VectorIndex>,
    embedder: Option<&dyn Embedder>,
) -> WamResult<()> {
    if let Term::Object(entries) = term {
        for (_, value) in entries {
            index_value_text(node_id, value, bm25, vector, embedder)?;
        }
    }
    Ok(())
}

fn index_value_text(
    node_id: &str,
    term: &Term,
    bm25: Option<&BM25Index>,
    vector: Option<&VectorIndex>,
    embedder: Option<&dyn Embedder>,
) -> WamResult<()> {
    match term {
        Term::Atom(value) | Term::String(value) => {
            index_string(node_id, value, bm25, vector, embedder)
        }
        Term::List(items) => index_items(node_id, items, bm25, vector, embedder),
        Term::Object(entries) => index_entries(node_id, entries, bm25, vector, embedder),
        _ => Ok(()),
    }
}

fn index_string(
    node_id: &str,
    value: &str,
    bm25: Option<&BM25Index>,
    vector: Option<&VectorIndex>,
    embedder: Option<&dyn Embedder>,
) -> WamResult<()> {
    if let Some(index) = bm25 {
        index.index_text(node_id, value).map_err(provider_error)?;
    }
    if let (Some(index), Some(embedder)) = (vector, embedder) {
        index
            .add_entry(VectorEntry {
                node_id: node_id.to_string(),
                embedding: embedder.embed(value)?,
                model: embedder.model().to_string(),
            })
            .map_err(provider_error)?;
    }
    Ok(())
}

fn index_items(
    node_id: &str,
    items: &[Term],
    bm25: Option<&BM25Index>,
    vector: Option<&VectorIndex>,
    embedder: Option<&dyn Embedder>,
) -> WamResult<()> {
    for item in items {
        index_value_text(node_id, item, bm25, vector, embedder)?;
    }
    Ok(())
}

fn index_entries(
    node_id: &str,
    entries: &[(String, Term)],
    bm25: Option<&BM25Index>,
    vector: Option<&VectorIndex>,
    embedder: Option<&dyn Embedder>,
) -> WamResult<()> {
    for (_, value) in entries {
        index_value_text(node_id, value, bm25, vector, embedder)?;
    }
    Ok(())
}

fn compound(term: &Term) -> Option<(&str, &[Term])> {
    match term {
        Term::Compound { name, args } => Some((name, args)),
        _ => None,
    }
}

fn atom(term: &Term) -> WamResult<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value),
        _ => Err(WamError::Provider("expected atom".to_string())),
    }
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
