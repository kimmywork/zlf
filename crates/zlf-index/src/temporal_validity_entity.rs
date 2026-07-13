use std::collections::BinaryHeap;

use rocksdb::{Direction, IteratorMode};
use zlf_core::{EntityRef, Result, ZlfError};

use crate::temporal_validity::ValidityStore;
use crate::temporal_validity_support::{
    entity_document_prefix, graph_entity_prefix, internal, result, serialization, validate_limit,
    HeapValidity,
};
use crate::{
    validate_half_open_range, GenerationId, IndexDocumentId, TemporalAccessPath,
    ValidityQueryResult, ValidityRecord,
};

impl ValidityStore {
    pub fn for_document(
        &self,
        generation: &GenerationId,
        document_id: &IndexDocumentId,
        limit: usize,
    ) -> Result<ValidityQueryResult> {
        validate_limit(generation, limit)?;
        let prefix = entity_document_prefix(generation, document_id);
        self.entity_records(&prefix, limit, |_| true)
    }

    pub fn for_entity(
        &self,
        generation: &GenerationId,
        entity: &EntityRef,
        limit: usize,
    ) -> Result<ValidityQueryResult> {
        self.for_entity_matching(generation, entity, limit, |_| true)
    }

    pub fn valid_at_for_entity(
        &self,
        generation: &GenerationId,
        entity: &EntityRef,
        instant: i64,
        limit: usize,
    ) -> Result<ValidityQueryResult> {
        self.for_entity_matching(generation, entity, limit, |record| record.contains(instant))
    }

    pub fn overlaps_for_entity(
        &self,
        generation: &GenerationId,
        entity: &EntityRef,
        start: i64,
        end: i64,
        limit: usize,
    ) -> Result<ValidityQueryResult> {
        validate_half_open_range(start, end).map_err(ZlfError::Internal)?;
        self.for_entity_matching(generation, entity, limit, |record| {
            record.overlaps(start, end)
        })
    }

    fn for_entity_matching(
        &self,
        generation: &GenerationId,
        entity: &EntityRef,
        limit: usize,
        matches: impl Fn(&ValidityRecord) -> bool,
    ) -> Result<ValidityQueryResult> {
        validate_limit(generation, limit)?;
        let prefix = graph_entity_prefix(generation, entity);
        self.entity_records(&prefix, limit, matches)
    }

    fn entity_records(
        &self,
        prefix: &[u8],
        limit: usize,
        matches: impl Fn(&ValidityRecord) -> bool,
    ) -> Result<ValidityQueryResult> {
        let mut heap = BinaryHeap::new();
        let mut candidates = 0;
        for item in self
            .db
            .iterator(IteratorMode::From(prefix, Direction::Forward))
        {
            let (key, value) = item.map_err(internal)?;
            if !key.starts_with(prefix) {
                break;
            }
            candidates += 1;
            let record = bincode::deserialize(&value).map_err(serialization)?;
            if matches(&record) {
                heap.push(HeapValidity(record));
                if heap.len() > limit {
                    heap.pop();
                }
            }
        }
        Ok(result(heap, candidates, TemporalAccessPath::ValidByStart))
    }
}
