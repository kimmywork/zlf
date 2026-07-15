use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};

use zlf_core::{Result, ZlfError};

use crate::{IndexDocumentId, VectorHit, VectorMetric, VectorRecord};

#[derive(Clone, Copy)]
pub(crate) struct DocumentFilters<'a> {
    pub includes: &'a HashSet<&'a IndexDocumentId>,
    pub excludes: &'a HashSet<&'a IndexDocumentId>,
    pub include_entities: &'a HashSet<&'a zlf_core::EntityRef>,
    pub exclude_entities: &'a HashSet<&'a zlf_core::EntityRef>,
}

#[derive(Debug)]
pub(crate) struct HeapHit(pub VectorHit);

impl PartialEq for HeapHit {
    fn eq(&self, other: &Self) -> bool {
        self.0.score == other.0.score && self.0.key == other.0.key
    }
}
impl Eq for HeapHit {}
impl PartialOrd for HeapHit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HeapHit {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .score
            .total_cmp(&self.0.score)
            .then_with(|| self.0.key.cmp(&other.0.key))
    }
}

pub(crate) fn similarity(query: &[f32], vector: &[f32], metric: VectorMetric) -> Result<f32> {
    let dot = query
        .iter()
        .zip(vector)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum::<f64>();
    let score = match metric {
        VectorMetric::DotProduct => dot,
        VectorMetric::Cosine => {
            let query_norm = norm(query);
            let vector_norm = norm(vector);
            if query_norm == 0.0 || vector_norm == 0.0 {
                return Err(ZlfError::Internal("cosine vector must be nonzero".into()));
            }
            dot / (query_norm * vector_norm)
        }
    };
    Ok(score as f32)
}

fn norm(values: &[f32]) -> f64 {
    values
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt()
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
