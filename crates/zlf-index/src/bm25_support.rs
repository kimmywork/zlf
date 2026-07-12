use std::collections::BTreeMap;

use tantivy::query::{Bm25StatisticsProvider, BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TantivyDocument, Value, STORED, STRING, TEXT,
};
use tantivy::Term;
use zlf_core::{EntityRef, Result, ZlfError};

use crate::{bm25_term_score, Bm25Config, Bm25Explanation, IndexDocumentId, TermScoreExplanation};

#[derive(Clone, Copy)]
pub(crate) struct Fields {
    pub key: Field,
    pub entity_kind: Field,
    pub entity_id: Field,
    pub field: Field,
    pub chunk: Field,
    pub language: Field,
    pub body: Field,
}

pub(crate) struct DocumentParts<'a> {
    pub key: &'a str,
    pub entity_kind: &'a str,
    pub entity_id: &'a str,
    pub field: &'a str,
    pub chunk: &'a str,
    pub language: &'a str,
    pub text: &'a str,
}

pub(crate) fn schema() -> (Schema, Fields) {
    let mut builder = Schema::builder();
    let fields = Fields {
        key: builder.add_text_field("document_key", STRING),
        entity_kind: builder.add_text_field("entity_kind", STRING | STORED),
        entity_id: builder.add_text_field("entity_id", STRING | STORED),
        field: builder.add_text_field("field", STRING | STORED),
        chunk: builder.add_text_field("chunk", STRING | STORED),
        language: builder.add_text_field("language", STRING | STORED),
        body: builder.add_text_field("body", TEXT | STORED),
    };
    (builder.build(), fields)
}

pub(crate) fn validate_schema(schema: &Schema) -> Result<()> {
    for field in [
        "document_key",
        "entity_kind",
        "entity_id",
        "field",
        "chunk",
        "language",
        "body",
    ] {
        schema.get_field(field).map_err(internal)?;
    }
    Ok(())
}

pub(crate) fn combined_query(
    fields: Fields,
    terms: &[String],
    field_filters: &[String],
    language_filters: &[String],
) -> Box<dyn Query> {
    let mut clauses = vec![(Occur::Must, term_query(fields.body, terms))];
    if !field_filters.is_empty() {
        clauses.push((Occur::Must, filter_query(fields.field, field_filters)));
    }
    if !language_filters.is_empty() {
        clauses.push((Occur::Must, filter_query(fields.language, language_filters)));
    }
    Box::new(BooleanQuery::new(clauses))
}

fn filter_query(field: Field, filters: &[String]) -> Box<dyn Query> {
    Box::new(BooleanQuery::new(
        filters
            .iter()
            .map(|value| {
                (
                    Occur::Should,
                    Box::new(TermQuery::new(
                        Term::from_field_text(field, value),
                        IndexRecordOption::Basic,
                    )) as Box<dyn Query>,
                )
            })
            .collect(),
    ))
}

pub(crate) fn entity_parts(entity: &EntityRef) -> (&'static str, &str) {
    match entity {
        EntityRef::Node(id) => ("node", id),
        EntityRef::Edge(id) => ("edge", id),
    }
}

pub(crate) fn document_key(id: &IndexDocumentId) -> String {
    id.canonical_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn stored_entity(document: &TantivyDocument, fields: Fields) -> Result<EntityRef> {
    let id = stored_text(document, fields.entity_id)?;
    match stored_text(document, fields.entity_kind)?.as_str() {
        "node" => Ok(EntityRef::Node(id)),
        "edge" => Ok(EntityRef::Edge(id)),
        kind => Err(ZlfError::Internal(format!(
            "invalid BM25 entity kind: {kind}"
        ))),
    }
}

pub(crate) fn stored_text(document: &TantivyDocument, field: Field) -> Result<String> {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| ZlfError::Internal("BM25 stored field is missing".into()))
}

pub(crate) fn bm25_explanation(
    searcher: &tantivy::Searcher,
    body_field: Field,
    terms: &[String],
    body: &str,
    field_weight: f32,
) -> Result<Bm25Explanation> {
    let tokens = body.split_whitespace().collect::<Vec<_>>();
    let document_count = searcher.num_docs();
    let average_document_length = if document_count == 0 {
        0.0
    } else {
        searcher.total_num_tokens(body_field).map_err(internal)? as f32 / document_count as f32
    };
    let stats = ExplanationStats {
        document_count,
        document_length: tokens.len() as u64,
        average_document_length,
        field_weight,
    };
    let components = terms
        .iter()
        .map(|term| term_component(searcher, body_field, term, &tokens, stats))
        .collect::<Result<Vec<_>>>()?;
    Ok(Bm25Explanation {
        terms: components,
        document_length: stats.document_length,
        average_document_length,
        field_weight,
    })
}

#[derive(Clone, Copy)]
struct ExplanationStats {
    document_count: u64,
    document_length: u64,
    average_document_length: f32,
    field_weight: f32,
}

fn term_component(
    searcher: &tantivy::Searcher,
    body_field: Field,
    term: &str,
    tokens: &[&str],
    stats: ExplanationStats,
) -> Result<TermScoreExplanation> {
    let frequency = tokens.iter().filter(|token| **token == term).count() as u64;
    let document_frequency = searcher
        .doc_freq(&Term::from_field_text(body_field, term))
        .map_err(internal)?;
    let numerator = (stats.document_count - document_frequency) as f64 + 0.5;
    let idf = (1.0 + numerator / (document_frequency as f64 + 0.5)).ln();
    let config = Bm25Config::default();
    let score = bm25_term_score(
        frequency,
        document_frequency,
        stats.document_count,
        stats.document_length,
        stats.average_document_length.into(),
        config.k1.into(),
        config.b.into(),
    );
    Ok(TermScoreExplanation {
        term: term.into(),
        term_frequency: frequency,
        document_frequency,
        inverse_document_frequency: idf as f32,
        score: score as f32 * stats.field_weight,
    })
}

pub(crate) fn validate_weights(weights: &BTreeMap<String, f32>) -> Result<()> {
    if weights
        .values()
        .any(|weight| !weight.is_finite() || *weight <= 0.0)
    {
        Err(ZlfError::Internal(
            "BM25 field weights must be positive and finite".into(),
        ))
    } else {
        Ok(())
    }
}

fn term_query(field: Field, terms: &[String]) -> Box<dyn Query> {
    Box::new(BooleanQuery::new(
        terms
            .iter()
            .map(|token| {
                (
                    Occur::Should,
                    Box::new(TermQuery::new(
                        Term::from_field_text(field, token),
                        IndexRecordOption::WithFreqs,
                    )) as Box<dyn Query>,
                )
            })
            .collect(),
    ))
}

pub(crate) fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}
