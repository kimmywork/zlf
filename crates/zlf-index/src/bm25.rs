use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use tantivy::collector::TopDocs;
use tantivy::query::Query;
use tantivy::schema::{Schema, TantivyDocument};
use tantivy::{doc, DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, Term};
use zlf_core::{EntityRef, Result, ZlfError};

use crate::bm25_support::{
    combined_query, document_key, entity_parts, schema, stored_text, validate_schema,
    validate_weights, DocumentParts, Fields,
};
use crate::{IndexDocument, IndexDocumentId, UnicodeJiebaAnalyzer};

const DEFAULT_TOP_K: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct BM25DocumentHit {
    pub document_id: IndexDocumentId,
    pub score: f32,
    pub explanation: Option<String>,
}

pub struct BM25Index {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    fields: Fields,
    analyzer: UnicodeJiebaAnalyzer,
}

impl BM25Index {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        std::fs::create_dir_all(path)?;
        let (schema, fields) = schema();
        let index = if path.join("meta.json").exists() {
            Index::open_in_dir(path).map_err(internal)?
        } else {
            Index::create_in_dir(path, schema).map_err(internal)?
        };
        validate_schema(&index.schema())?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .map_err(internal)?;
        let writer = index.writer(50_000_000).map_err(internal)?;
        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
            analyzer: UnicodeJiebaAnalyzer::default(),
        })
    }

    pub fn tokenize(&self, text: &str) -> Vec<String> {
        self.analyzer.analyze(text)
    }

    pub fn index_text(&self, document_id: &str, text: &str) -> Result<()> {
        self.index_texts_batch(&[(document_id, text)])
    }

    pub fn index_texts_batch(&self, documents: &[(&str, &str)]) -> Result<()> {
        let mut writer = self.writer.lock().map_err(internal)?;
        for (id, text) in documents {
            self.write_document(
                &mut writer,
                DocumentParts {
                    key: id,
                    entity_kind: "node",
                    entity_id: id,
                    field: "_all",
                    chunk: "0",
                    text,
                },
            )?;
        }
        self.commit(&mut writer)
    }

    pub fn index_document(&self, document: &IndexDocument) -> Result<()> {
        let (kind, entity_id) = entity_parts(&document.id.entity);
        let mut writer = self.writer.lock().map_err(internal)?;
        let key = document_key(&document.id);
        self.write_document(
            &mut writer,
            DocumentParts {
                key: &key,
                entity_kind: kind,
                entity_id,
                field: &document.id.field,
                chunk: &document.id.chunk_id,
                text: &document.content,
            },
        )?;
        self.commit(&mut writer)
    }

    pub fn remove_document(&self, id: &IndexDocumentId) -> Result<()> {
        self.remove_key(&document_key(id))
    }

    pub fn remove_all_for_node(&self, document_id: &str) -> Result<()> {
        self.remove_key(document_id)
    }

    pub fn search(&self, query: &str) -> Result<Vec<(String, f32)>> {
        Ok(self
            .search_document_top_k(query, DEFAULT_TOP_K, &[], &BTreeMap::new(), false)?
            .into_iter()
            .map(|hit| (hit.document_id.entity.id().to_string(), hit.score))
            .collect())
    }

    pub fn search_top_k(&self, query: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        Ok(self
            .search_document_top_k(query, top_k, &[], &BTreeMap::new(), false)?
            .into_iter()
            .map(|hit| (hit.document_id.entity.id().to_string(), hit.score))
            .collect())
    }

    pub fn search_document_top_k(
        &self,
        query: &str,
        top_k: usize,
        fields: &[String],
        field_weights: &BTreeMap<String, f32>,
        explain: bool,
    ) -> Result<Vec<BM25DocumentHit>> {
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let terms = self.tokenize(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        validate_weights(field_weights)?;
        let query = combined_query(self.fields, &terms, fields);
        let mut results = self.collect_hits(
            query.as_ref(),
            top_k.saturating_mul(8).max(top_k).min(10_000),
            field_weights,
            explain,
        )?;
        results.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.document_id.cmp(&right.document_id))
        });
        results.truncate(top_k);
        Ok(results)
    }

    pub fn document_count(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    pub fn schema(&self) -> Schema {
        self.index.schema()
    }

    fn collect_hits(
        &self,
        query: &dyn Query,
        candidate_limit: usize,
        field_weights: &BTreeMap<String, f32>,
        explain: bool,
    ) -> Result<Vec<BM25DocumentHit>> {
        let searcher = self.reader.searcher();
        searcher
            .search(query, &TopDocs::with_limit(candidate_limit))
            .map_err(internal)?
            .into_iter()
            .map(|(score, address)| {
                self.document_hit(&searcher, query, address, score, field_weights, explain)
            })
            .collect()
    }

    fn write_document(&self, writer: &mut IndexWriter, parts: DocumentParts<'_>) -> Result<()> {
        writer.delete_term(Term::from_field_text(self.fields.key, parts.key));
        writer
            .add_document(doc!(
                self.fields.key => parts.key,
                self.fields.entity_kind => parts.entity_kind,
                self.fields.entity_id => parts.entity_id,
                self.fields.field => parts.field,
                self.fields.chunk => parts.chunk,
                self.fields.body => self.tokenize(parts.text).join(" ")
            ))
            .map(|_| ())
            .map_err(internal)
    }

    fn remove_key(&self, key: &str) -> Result<()> {
        let mut writer = self.writer.lock().map_err(internal)?;
        writer.delete_term(Term::from_field_text(self.fields.key, key));
        self.commit(&mut writer)
    }

    fn commit(&self, writer: &mut IndexWriter) -> Result<()> {
        writer.commit().map_err(internal)?;
        self.reader.reload().map_err(internal)
    }

    fn document_hit(
        &self,
        searcher: &tantivy::Searcher,
        query: &dyn Query,
        address: DocAddress,
        score: f32,
        weights: &BTreeMap<String, f32>,
        explain: bool,
    ) -> Result<BM25DocumentHit> {
        let document = searcher.doc::<TantivyDocument>(address).map_err(internal)?;
        let text = |field| stored_text(&document, field);
        let field = text(self.fields.field)?;
        let entity = match text(self.fields.entity_kind)?.as_str() {
            "node" => EntityRef::Node(text(self.fields.entity_id)?),
            "edge" => EntityRef::Edge(text(self.fields.entity_id)?),
            kind => {
                return Err(ZlfError::Internal(format!(
                    "invalid BM25 entity kind: {kind}"
                )))
            }
        };
        let explanation = explain
            .then(|| {
                query
                    .explain(searcher, address)
                    .map(|value| format!("{value:?}"))
            })
            .transpose()
            .map_err(internal)?;
        Ok(BM25DocumentHit {
            document_id: IndexDocumentId::new(entity, field.clone(), text(self.fields.chunk)?),
            score: score * weights.get(&field).copied().unwrap_or(1.0),
            explanation,
        })
    }
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}
