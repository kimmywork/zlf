use std::path::Path;
use std::sync::Mutex;

use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TantivyDocument, Value, STORED, STRING, TEXT,
};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, Term};
use zlf_core::{Result, ZlfError};

use crate::UnicodeJiebaAnalyzer;

const DEFAULT_TOP_K: usize = 100;

pub struct BM25Index {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    id_field: Field,
    body_field: Field,
    analyzer: UnicodeJiebaAnalyzer,
}

impl BM25Index {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        std::fs::create_dir_all(path)?;
        let (schema, id_field, body_field) = schema();
        let index = if path.join("meta.json").exists() {
            Index::open_in_dir(path).map_err(internal)?
        } else {
            Index::create_in_dir(path, schema).map_err(internal)?
        };
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
            id_field,
            body_field,
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
            writer.delete_term(Term::from_field_text(self.id_field, id));
            writer
                .add_document(doc!(
                    self.id_field => *id,
                    self.body_field => self.tokenize(text).join(" ")
                ))
                .map_err(internal)?;
        }
        writer.commit().map_err(internal)?;
        self.reader.reload().map_err(internal)
    }

    pub fn search(&self, query: &str) -> Result<Vec<(String, f32)>> {
        self.search_top_k(query, DEFAULT_TOP_K)
    }

    pub fn search_top_k(&self, query: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        if top_k == 0 {
            return Ok(Vec::new());
        }
        let terms = self.tokenize(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let query = term_query(self.body_field, &terms);
        let searcher = self.reader.searcher();
        let hits = searcher
            .search(query.as_ref(), &TopDocs::with_limit(top_k))
            .map_err(internal)?;
        let mut results = hits
            .into_iter()
            .map(|(score, address)| {
                let document = searcher.doc::<TantivyDocument>(address).map_err(internal)?;
                let id = document
                    .get_first(self.id_field)
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| ZlfError::Internal("BM25 document has no ID".into()))?;
                Ok((id.to_string(), score))
            })
            .collect::<Result<Vec<_>>>()?;
        results.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        Ok(results)
    }

    pub fn remove_all_for_node(&self, document_id: &str) -> Result<()> {
        let mut writer = self.writer.lock().map_err(internal)?;
        writer.delete_term(Term::from_field_text(self.id_field, document_id));
        writer.commit().map_err(internal)?;
        self.reader.reload().map_err(internal)
    }

    pub fn document_count(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    pub fn schema(&self) -> Schema {
        self.index.schema()
    }
}

fn schema() -> (Schema, Field, Field) {
    let mut builder = Schema::builder();
    let id = builder.add_text_field("id", STRING | STORED);
    let body = builder.add_text_field("body", TEXT);
    (builder.build(), id, body)
}

fn term_query(field: Field, terms: &[String]) -> Box<dyn Query> {
    let clauses = terms
        .iter()
        .map(|token| {
            let query = TermQuery::new(
                Term::from_field_text(field, token),
                IndexRecordOption::WithFreqs,
            );
            (Occur::Should, Box::new(query) as Box<dyn Query>)
        })
        .collect();
    Box::new(BooleanQuery::new(clauses))
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> (BM25Index, tempfile::TempDir) {
        let temp = tempfile::tempdir().unwrap();
        let index = BM25Index::open(temp.path()).unwrap();
        (index, temp)
    }

    #[test]
    fn real_bm25_prefers_repeated_term_and_limits_results() {
        let (index, _temp) = index();
        index
            .index_texts_batch(&[
                ("alice", "rust rust database"),
                ("bob", "rust graph database"),
            ])
            .unwrap();
        let hits = index.search_top_k("rust", 1).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, "alice");
    }

    #[test]
    fn replacement_and_delete_remove_obsolete_terms() {
        let (index, _temp) = index();
        index.index_text("alice", "obsolete text").unwrap();
        index.index_text("alice", "current value").unwrap();
        assert!(index.search("obsolete").unwrap().is_empty());
        assert_eq!(index.search("current").unwrap()[0].0, "alice");
        index.remove_all_for_node("alice").unwrap();
        assert!(index.search("current").unwrap().is_empty());
    }

    #[test]
    fn chinese_mixed_tokenization_and_reopen_work() {
        let temp = tempfile::tempdir().unwrap();
        {
            let index = BM25Index::open(temp.path()).unwrap();
            index.index_text("alice", "Alice 是软件工程师").unwrap();
            assert_eq!(index.search("软件").unwrap()[0].0, "alice");
        }
        let reopened = BM25Index::open(temp.path()).unwrap();
        assert_eq!(reopened.search("Alice").unwrap()[0].0, "alice");
        assert_eq!(reopened.document_count(), 1);
    }

    #[test]
    fn equal_scores_use_stable_id_tie_break() {
        let (index, _temp) = index();
        index
            .index_texts_batch(&[("b", "same"), ("a", "same")])
            .unwrap();
        let ids = index
            .search("same")
            .unwrap()
            .into_iter()
            .map(|hit| hit.0)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["a", "b"]);
    }
}
