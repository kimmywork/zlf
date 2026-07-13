use std::collections::{BTreeMap, HashSet};

use crate::{IndexDocumentId, VectorRecord};

#[derive(Clone, Copy)]
pub(crate) struct DocumentFilters<'a> {
    pub includes: &'a HashSet<&'a IndexDocumentId>,
    pub excludes: &'a HashSet<&'a IndexDocumentId>,
    pub include_entities: &'a HashSet<&'a zlf_core::EntityRef>,
    pub exclude_entities: &'a HashSet<&'a zlf_core::EntityRef>,
}

pub(crate) fn matches_filters(
    record: &VectorRecord,
    filters: DocumentFilters<'_>,
    fields: &[String],
    metadata: &BTreeMap<String, String>,
) -> bool {
    (filters.includes.is_empty() || filters.includes.contains(&record.key.document_id))
        && !filters.excludes.contains(&record.key.document_id)
        && (filters.include_entities.is_empty()
            || filters
                .include_entities
                .contains(&record.key.document_id.entity))
        && !filters
            .exclude_entities
            .contains(&record.key.document_id.entity)
        && (fields.is_empty() || fields.contains(&record.key.document_id.field))
        && metadata
            .iter()
            .all(|(key, value)| record.metadata.get(key) == Some(value))
}
