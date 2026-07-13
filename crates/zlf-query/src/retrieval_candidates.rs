use std::collections::BTreeMap;

use zlf_core::{Result, ZlfError};
use zlf_index::{
    reciprocal_rank_fusion, IndexDocumentId, IndexPageRequest, RankedRetrieverHit,
    ResultAggregation, RetrievalHit, RetrievalMode, RetrievalQuery, VectorKey, VectorQuery,
    DEFAULT_RRF_K,
};

use crate::{PreparedRetrieval, RetrievalExecutionMetadata, ZlfDatabase};

#[derive(Default)]
struct CandidateResult {
    hits: Vec<RankedRetrieverHit>,
    pages: usize,
    exhausted: bool,
}

impl ZlfDatabase {
    pub(crate) fn fused_candidates(
        &self,
        prepared: &PreparedRetrieval,
        bound_entity: Option<&zlf_core::EntityRef>,
    ) -> Result<(Vec<RetrievalHit>, RetrievalExecutionMetadata)> {
        let lexical = self.lexical_candidates(prepared, bound_entity)?;
        let vector = self.vector_candidates(prepared, bound_entity)?;
        let lexical_pages = lexical.pages;
        let vector_pages = vector.pages;
        let exhausted = lexical.exhausted || vector.exhausted;
        let (lexical_hits, vector_hits) = match prepared.request.aggregation {
            ResultAggregation::Document => (lexical.hits, vector.hits),
            ResultAggregation::Entity => (
                aggregate_entities(lexical.hits),
                aggregate_entities(vector.hits),
            ),
        };
        let metadata = candidate_metadata(
            prepared,
            bound_entity,
            lexical_pages,
            vector_pages,
            lexical_hits.len(),
            vector_hits.len(),
            exhausted,
        );
        let fused = reciprocal_rank_fusion(
            &lexical_hits,
            &vector_hits,
            prepared.request.budgets.candidate_k,
            DEFAULT_RRF_K,
        )
        .map_err(|error| ZlfError::Internal(error.to_string()))?;
        Ok((fused, metadata))
    }

    fn lexical_candidates(
        &self,
        prepared: &PreparedRetrieval,
        bound_entity: Option<&zlf_core::EntityRef>,
    ) -> Result<CandidateResult> {
        if !matches!(
            prepared.request.mode,
            RetrievalMode::Lexical | RetrievalMode::Hybrid
        ) {
            return Ok(CandidateResult::default());
        }
        let RetrievalQuery::Text { text } = &prepared.request.query else {
            return Err(ZlfError::Internal(
                "lexical retrieval requires a prepared text query".into(),
            ));
        };
        let index = self
            .bm25
            .read()
            .map_err(crate::helpers::lock_error)?
            .clone();
        collect_lexical_pages(index.as_ref(), prepared, text, bound_entity)
    }

    fn vector_candidates(
        &self,
        prepared: &PreparedRetrieval,
        bound_entity: Option<&zlf_core::EntityRef>,
    ) -> Result<CandidateResult> {
        if !matches!(
            prepared.request.mode,
            RetrievalMode::Vector | RetrievalMode::Hybrid
        ) {
            return Ok(CandidateResult::default());
        }
        let query = self.vector_query(prepared, bound_entity)?;
        let mut result = CandidateResult::default();
        let mut offset = 0;
        loop {
            let page = self.vector.search_page(
                &query,
                &self.vector_model,
                page_request(prepared, offset),
            )?;
            result.pages += 1;
            result.exhausted |= page.candidate_budget_exhausted;
            result
                .hits
                .extend(page.items.into_iter().map(|hit| RankedRetrieverHit {
                    document_id: hit.key.document_id,
                    score: hit.score,
                    generation: hit.key.generation,
                    watermark: prepared.snapshot.vector_watermark,
                    source_range: None,
                }));
            let Some(next) = page.next_offset else { break };
            offset = next;
        }
        Ok(result)
    }

    fn vector_query(
        &self,
        prepared: &PreparedRetrieval,
        bound_entity: Option<&zlf_core::EntityRef>,
    ) -> Result<VectorQuery> {
        Ok(VectorQuery {
            generation: prepared.snapshot.vector_generation.clone(),
            model_profile: prepared.snapshot.model_id.clone(),
            model_version: prepared.snapshot.model_version,
            values: self.prepared_vector(prepared)?,
            top_k: prepared.request.budgets.candidate_k,
            threshold: prepared.request.threshold,
            include_sources: Vec::new(),
            exclude_sources: prepared
                .request
                .exclude_source
                .clone()
                .into_iter()
                .collect(),
            include_entities: bound_entity.cloned().into_iter().collect(),
            exclude_entities: Vec::new(),
            fields: prepared.request.fields.clone(),
            metadata: BTreeMap::new(),
        })
    }

    fn prepared_vector(&self, prepared: &PreparedRetrieval) -> Result<Vec<f32>> {
        if let Some(values) = &prepared.query_vector {
            return Ok(values.clone());
        }
        let RetrievalQuery::SourceDocument { document_id } = &prepared.request.query else {
            return Err(ZlfError::Internal(
                "vector retrieval requires a prepared vector or source document".into(),
            ));
        };
        let key = VectorKey {
            generation: prepared.snapshot.vector_generation.clone(),
            model_profile: prepared.snapshot.model_id.clone(),
            model_version: prepared.snapshot.model_version,
            document_id: document_id.clone(),
        };
        self.vector
            .get(&key)?
            .map(|record| record.values)
            .ok_or_else(|| ZlfError::Internal("source document vector is not indexed".into()))
    }
}

fn candidate_metadata(
    prepared: &PreparedRetrieval,
    bound_entity: Option<&zlf_core::EntityRef>,
    lexical_pages: usize,
    vector_pages: usize,
    lexical_candidates: usize,
    vector_candidates: usize,
    exhausted: bool,
) -> RetrievalExecutionMetadata {
    RetrievalExecutionMetadata {
        mode: format!("{:?}", prepared.request.mode).to_lowercase(),
        strategy: if bound_entity.is_some() {
            "bound_entity".into()
        } else {
            "retrieval_first".into()
        },
        lexical_pages,
        vector_pages,
        lexical_candidates,
        vector_candidates,
        candidate_budget_exhausted: exhausted,
        ..RetrievalExecutionMetadata::default()
    }
}

fn collect_lexical_pages(
    index: &zlf_index::BM25Index,
    prepared: &PreparedRetrieval,
    text: &str,
    bound_entity: Option<&zlf_core::EntityRef>,
) -> Result<CandidateResult> {
    let entities = bound_entity
        .map(|entity| vec![entity.id().to_string()])
        .unwrap_or_default();
    let mut result = CandidateResult::default();
    let mut offset = 0;
    loop {
        let page = index.search_document_page_for_entities(
            text,
            page_request(prepared, offset),
            &prepared.request.fields,
            &entities,
            &BTreeMap::new(),
            prepared.request.explain,
        )?;
        result.pages += 1;
        result.exhausted |= page.candidate_budget_exhausted;
        result.hits.extend(
            page.items
                .into_iter()
                .filter(|hit| prepared.request.exclude_source.as_ref() != Some(&hit.document_id))
                .map(|hit| lexical_hit(hit, prepared)),
        );
        let Some(next) = page.next_offset else { break };
        offset = next;
    }
    Ok(result)
}

fn lexical_hit(
    hit: zlf_index::BM25DocumentHit,
    prepared: &PreparedRetrieval,
) -> RankedRetrieverHit {
    RankedRetrieverHit {
        document_id: hit.document_id,
        score: hit.score,
        generation: prepared.snapshot.lexical_generation.clone(),
        watermark: prepared.snapshot.lexical_watermark,
        source_range: None,
    }
}

fn page_request(prepared: &PreparedRetrieval, offset: usize) -> IndexPageRequest {
    IndexPageRequest {
        offset,
        page_size: prepared.request.budgets.page_size,
        candidate_limit: prepared.request.budgets.candidate_k,
    }
}

fn aggregate_entities(hits: Vec<RankedRetrieverHit>) -> Vec<RankedRetrieverHit> {
    let mut seen = std::collections::BTreeSet::new();
    hits.into_iter()
        .filter_map(|mut hit| {
            seen.insert(hit.document_id.entity.clone()).then(|| {
                hit.document_id =
                    IndexDocumentId::new(hit.document_id.entity.clone(), "_entity", "0");
                hit
            })
        })
        .collect()
}
