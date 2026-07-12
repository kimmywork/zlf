use std::collections::BTreeMap;

use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TantivyDocument, Value, STORED, STRING, TEXT,
};
use tantivy::Term;
use zlf_core::{EntityRef, Result, ZlfError};

use crate::IndexDocumentId;

#[derive(Clone, Copy)]
pub(crate) struct Fields {
    pub key: Field,
    pub entity_kind: Field,
    pub entity_id: Field,
    pub field: Field,
    pub chunk: Field,
    pub body: Field,
}

pub(crate) struct DocumentParts<'a> {
    pub key: &'a str,
    pub entity_kind: &'a str,
    pub entity_id: &'a str,
    pub field: &'a str,
    pub chunk: &'a str,
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
        body: builder.add_text_field("body", TEXT),
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
        "body",
    ] {
        schema.get_field(field).map_err(internal)?;
    }
    Ok(())
}

pub(crate) fn combined_query(
    fields: Fields,
    terms: &[String],
    filters: &[String],
) -> Box<dyn Query> {
    let body = term_query(fields.body, terms);
    if filters.is_empty() {
        return body;
    }
    let field_query = BooleanQuery::new(
        filters
            .iter()
            .map(|field| {
                (
                    Occur::Should,
                    Box::new(TermQuery::new(
                        Term::from_field_text(fields.field, field),
                        IndexRecordOption::Basic,
                    )) as Box<dyn Query>,
                )
            })
            .collect(),
    );
    Box::new(BooleanQuery::new(vec![
        (Occur::Must, body),
        (Occur::Must, Box::new(field_query)),
    ]))
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

pub(crate) fn stored_text(document: &TantivyDocument, field: Field) -> Result<String> {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| ZlfError::Internal("BM25 stored field is missing".into()))
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

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}
