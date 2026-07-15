use zlf_index::{
    BM25Index, EmbeddingModelProfile, EventTimeStore, ExactVectorStore, GenerationId,
    ValidityStore, VectorSearchBackend,
};

use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_provider::FactProvider;
use super::index_limits::{IndexAnswerLimits, IndexAnswerMetrics, IndexAnswerState};
use super::index_vector_provider::ExactVectorProvider;
use super::predicate::PredicateKey;

#[derive(Default)]
pub struct IndexFactProvider<'a> {
    bm25: Option<&'a BM25Index>,
    vector: Option<ExactVectorProvider<'a>>,
    pub(super) temporal: Option<TemporalProvider<'a>>,
    pub(super) limits: IndexAnswerLimits,
    pub(super) answer_state: IndexAnswerState,
}

impl<'a> IndexFactProvider<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limits(mut self, limits: IndexAnswerLimits) -> WamResult<Self> {
        limits.validate()?;
        self.limits = limits;
        Ok(self)
    }

    pub fn answer_metrics(&self) -> IndexAnswerMetrics {
        self.answer_state.snapshot()
    }

    pub fn with_bm25(mut self, bm25: &'a BM25Index) -> Self {
        self.bm25 = Some(bm25);
        self
    }

    pub fn with_exact_vector(
        self,
        store: &'a ExactVectorStore,
        profile: &'a EmbeddingModelProfile,
        generation: &'a GenerationId,
    ) -> Self {
        self.with_vector_backend(store, profile, generation)
    }

    pub fn with_vector_backend(
        mut self,
        store: &'a dyn VectorSearchBackend,
        profile: &'a EmbeddingModelProfile,
        generation: &'a GenerationId,
    ) -> Self {
        self.vector = Some(ExactVectorProvider::new(store, profile, generation));
        self
    }

    pub fn with_temporal(
        mut self,
        events: &'a EventTimeStore,
        validities: &'a ValidityStore,
        generation: &'a GenerationId,
    ) -> Self {
        self.temporal = Some(TemporalProvider {
            events,
            validities,
            generation,
        });
        self
    }
}

impl FactProvider for IndexFactProvider<'_> {
    fn facts_for(&self, _key: &PredicateKey) -> WamResult<Vec<Term>> {
        Ok(Vec::new())
    }

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        match compound(goal)? {
            Some(("bm25", args)) => self.bm25_facts(args),
            Some(("vector_similar", args)) => self.vector_facts(args),
            Some(("temporal_on", args)) => self.temporal_on_facts(args),
            Some(("temporal_between", args)) => self.temporal_between_facts(args),
            Some(("valid_at", args)) => self.valid_at_facts(args),
            Some(("valid_overlaps", args)) => self.valid_overlaps_facts(args),
            _ => Ok(Vec::new()),
        }
    }
}

impl IndexFactProvider<'_> {
    fn bm25_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [query, target, _] = args else {
            return Ok(Vec::new());
        };
        let query = constant(query)?;
        let Some(index) = self.bm25 else {
            return Ok(Vec::new());
        };
        let entities = bound_constant(target)
            .map(|target| vec![target.to_string()])
            .unwrap_or_default();
        let candidates = index
            .search_document_top_k_for_entities(
                query,
                self.limits.candidate_limit,
                &[],
                &entities,
                &std::collections::BTreeMap::new(),
                false,
            )
            .map_err(provider_error)?
            .into_iter()
            .map(|hit| bm25_term(query, hit))
            .collect::<Vec<_>>();
        Ok(self.finish(candidates))
    }

    fn vector_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [source, target, _] = args else {
            return Ok(Vec::new());
        };
        let source = constant(source)?;
        let Some(index) = self.vector else {
            return Ok(Vec::new());
        };
        let (candidates, exhausted) =
            index.source_facts(source, bound_constant(target), self.limits.candidate_limit)?;
        Ok(self.answer_state.finish(candidates, self.limits, exhausted))
    }

    pub(super) fn finish(&self, candidates: Vec<Term>) -> Vec<Term> {
        let exhausted = candidates.len() == self.limits.candidate_limit;
        self.answer_state.finish(candidates, self.limits, exhausted)
    }
}

#[derive(Clone, Copy)]
pub(super) struct TemporalProvider<'a> {
    pub(super) events: &'a EventTimeStore,
    pub(super) validities: &'a ValidityStore,
    pub(super) generation: &'a GenerationId,
}

fn compound(term: &Term) -> WamResult<Option<(&str, &[Term])>> {
    match term {
        Term::Compound { name, args } => Ok(Some((name, args))),
        Term::Atom(_) => Ok(None),
        _ => Err(WamError::Provider("expected callable term".to_string())),
    }
}

fn bm25_term(query: &str, hit: zlf_index::BM25DocumentHit) -> Term {
    compound_term(
        "bm25",
        vec![
            string(query),
            atom(hit.document_id.entity.id()),
            number(hit.score),
        ],
    )
}

pub(super) fn bound_entity(term: &Term) -> Option<zlf_core::EntityRef> {
    bound_constant(term).map(|id| zlf_core::EntityRef::Node(id.into()))
}

fn bound_constant(term: &Term) -> Option<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Some(value),
        _ => None,
    }
}

pub(super) fn constant(term: &Term) -> WamResult<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value),
        _ => Err(WamError::Provider("expected bound constant".to_string())),
    }
}

pub(super) fn atom(value: impl Into<String>) -> Term {
    Term::Atom(value.into())
}

pub(super) fn string(value: impl Into<String>) -> Term {
    Term::String(value.into())
}

fn number(value: f32) -> Term {
    Term::Float(value as f64)
}

pub(super) fn compound_term(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}

pub(super) fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
