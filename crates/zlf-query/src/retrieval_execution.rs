use serde::{Deserialize, Serialize};
use zlf_core::{EntityRef, Result, ZlfError};
use zlf_index::{RetrievalHit, TemporalFilter};

use crate::{PreparedRetrieval, PreparedRetrievalHandle, ZlfDatabase};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalExecutionMetadata {
    pub mode: String,
    pub strategy: String,
    pub lexical_pages: usize,
    pub vector_pages: usize,
    pub lexical_candidates: usize,
    pub vector_candidates: usize,
    pub fused_candidates: usize,
    pub graph_rejected: usize,
    pub temporal_rejected: usize,
    pub candidate_budget_exhausted: bool,
    pub exact_filtered_top_k: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalExecutionResult {
    pub hits: Vec<RetrievalHit>,
    pub metadata: RetrievalExecutionMetadata,
}

impl ZlfDatabase {
    pub fn execute_prepared_retrieval(
        &self,
        handle: &PreparedRetrievalHandle,
    ) -> Result<RetrievalExecutionResult> {
        self.execute_prepared_retrieval_for_entity(handle, None)
    }

    pub fn execute_prepared_retrieval_for_entity(
        &self,
        handle: &PreparedRetrievalHandle,
        entity: Option<&EntityRef>,
    ) -> Result<RetrievalExecutionResult> {
        let prepared = self
            .prepared_retrieval(handle)
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        let (fused, mut metadata) = self.fused_candidates(&prepared, entity)?;
        metadata.fused_candidates = fused.len();
        let hits = self.filter_candidates(&prepared, fused, &mut metadata)?;
        metadata.exact_filtered_top_k = !metadata.candidate_budget_exhausted;
        Ok(RetrievalExecutionResult { hits, metadata })
    }

    fn filter_candidates(
        &self,
        prepared: &PreparedRetrieval,
        candidates: Vec<RetrievalHit>,
        metadata: &mut RetrievalExecutionMetadata,
    ) -> Result<Vec<RetrievalHit>> {
        let mut accepted = Vec::new();
        for mut hit in candidates {
            if !self.matches_temporal(prepared, &hit.document_id.entity)? {
                metadata.temporal_rejected += 1;
                continue;
            }
            if !self.matches_graph_filter(prepared, &hit.document_id.entity)? {
                metadata.graph_rejected += 1;
                continue;
            }
            hit.fused_rank = accepted.len() + 1;
            accepted.push(hit);
            if accepted.len() == prepared.request.top_k {
                break;
            }
        }
        Ok(accepted)
    }

    fn matches_temporal(&self, prepared: &PreparedRetrieval, entity: &EntityRef) -> Result<bool> {
        let Some(filter) = &prepared.request.temporal_filter else {
            return Ok(true);
        };
        let generation = &prepared.snapshot.temporal_generation;
        match *filter {
            TemporalFilter::EventRange {
                start_micros,
                end_micros,
            } => self
                .events
                .range_for_entity(generation, entity, start_micros, end_micros, 1)
                .map(|result| !result.records.is_empty()),
            TemporalFilter::ValidAt { instant_micros } => self
                .validities
                .valid_at_for_entity(generation, entity, instant_micros, 1)
                .map(|result| !result.records.is_empty()),
            TemporalFilter::ValidOverlaps {
                start_micros,
                end_micros,
            } => self
                .validities
                .overlaps_for_entity(generation, entity, start_micros, end_micros, 1)
                .map(|result| !result.records.is_empty()),
        }
    }

    fn matches_graph_filter(
        &self,
        prepared: &PreparedRetrieval,
        entity: &EntityRef,
    ) -> Result<bool> {
        let Some(source) = &prepared.request.graph_filter_goal else {
            return Ok(true);
        };
        let mut term = zlf_prolog::PrologParser::parse_term(source)?;
        bind_entity_variable(&mut term, entity);
        self.execute_terms(&[term])
            .map(|answers| !answers.is_empty())
    }
}

fn bind_entity_variable(term: &mut zlf_prolog::Term, entity: &EntityRef) {
    match term {
        zlf_prolog::Term::Variable(name) if name == "Entity" => {
            *term = zlf_prolog::Term::Atom(entity.id().into());
        }
        zlf_prolog::Term::Compound { args, .. } | zlf_prolog::Term::List(args) => {
            args.iter_mut()
                .for_each(|term| bind_entity_variable(term, entity));
        }
        zlf_prolog::Term::Object(entries) => entries
            .iter_mut()
            .for_each(|(_, term)| bind_entity_variable(term, entity)),
        _ => {}
    }
}
