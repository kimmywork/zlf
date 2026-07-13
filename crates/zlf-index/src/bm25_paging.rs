use std::collections::BTreeMap;

use zlf_core::{Result, ZlfError};

use crate::{ranked_page, BM25DocumentHit, BM25Index, IndexPage, IndexPageRequest};

impl BM25Index {
    pub fn search_document_page_for_entities(
        &self,
        query: &str,
        page: IndexPageRequest,
        fields: &[String],
        entity_ids: &[String],
        field_weights: &BTreeMap<String, f32>,
        explain: bool,
    ) -> Result<IndexPage<BM25DocumentHit>> {
        page.validate()
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        let hits = self.search_document_top_k_for_entities(
            query,
            page.probe_limit(),
            fields,
            entity_ids,
            field_weights,
            explain,
        )?;
        ranked_page(hits, page).map_err(|error| ZlfError::Internal(error.to_string()))
    }
}
