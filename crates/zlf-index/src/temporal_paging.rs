use zlf_core::{EntityRef, Result, ZlfError};

use crate::{
    ranked_page, EventRecord, EventTimeStore, GenerationId, IndexPage, IndexPageRequest,
    ValidityRecord, ValidityStore,
};

impl EventTimeStore {
    pub fn range_page(
        &self,
        generation: &GenerationId,
        start: i64,
        end: i64,
        page: IndexPageRequest,
    ) -> Result<IndexPage<EventRecord>> {
        page.validate()
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        let result = self.range(generation, start, end, page.probe_limit())?;
        temporal_page(result.records, result.candidates_scanned, page)
    }

    pub fn range_for_entity_page(
        &self,
        generation: &GenerationId,
        entity: &EntityRef,
        start: i64,
        end: i64,
        page: IndexPageRequest,
    ) -> Result<IndexPage<EventRecord>> {
        page.validate()
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        let result = self.range_for_entity(generation, entity, start, end, page.probe_limit())?;
        temporal_page(result.records, result.candidates_scanned, page)
    }
}

impl ValidityStore {
    pub fn valid_at_page(
        &self,
        generation: &GenerationId,
        instant: i64,
        page: IndexPageRequest,
    ) -> Result<IndexPage<ValidityRecord>> {
        page.validate()
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        let result = self.valid_at(generation, instant, page.probe_limit())?;
        temporal_page(result.records, result.candidates_scanned, page)
    }

    pub fn overlaps_page(
        &self,
        generation: &GenerationId,
        start: i64,
        end: i64,
        page: IndexPageRequest,
    ) -> Result<IndexPage<ValidityRecord>> {
        page.validate()
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        let result = self.overlaps(generation, start, end, page.probe_limit())?;
        temporal_page(result.records, result.candidates_scanned, page)
    }
}

fn temporal_page<T>(
    records: Vec<T>,
    candidates_scanned: u64,
    request: IndexPageRequest,
) -> Result<IndexPage<T>> {
    let mut page =
        ranked_page(records, request).map_err(|error| ZlfError::Internal(error.to_string()))?;
    page.candidates_scanned = candidates_scanned;
    Ok(page)
}
