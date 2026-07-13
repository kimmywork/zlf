use zlf_index::{
    parse_utc_micros, BM25Index, EmbeddingModelProfile, EventTimeStore, ExactVectorStore,
    GenerationId, ValidityStore,
};

use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_provider::FactProvider;
use super::index_vector_provider::ExactVectorProvider;
use super::predicate::PredicateKey;

#[derive(Default)]
pub struct IndexFactProvider<'a> {
    bm25: Option<&'a BM25Index>,
    vector: Option<ExactVectorProvider<'a>>,
    temporal: Option<TemporalProvider<'a>>,
}

impl<'a> IndexFactProvider<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_bm25(mut self, bm25: &'a BM25Index) -> Self {
        self.bm25 = Some(bm25);
        self
    }

    pub fn with_exact_vector(
        mut self,
        store: &'a ExactVectorStore,
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
        let [query, _, _] = args else {
            return Ok(Vec::new());
        };
        let query = constant(query)?;
        let Some(index) = self.bm25 else {
            return Ok(Vec::new());
        };
        Ok(index
            .search(query)
            .map_err(provider_error)?
            .into_iter()
            .map(|(node, score)| {
                compound_term("bm25", vec![string(query), atom(node), number(score)])
            })
            .collect())
    }

    fn vector_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [source, _, _] = args else {
            return Ok(Vec::new());
        };
        let source = constant(source)?;
        let Some(index) = self.vector else {
            return Ok(Vec::new());
        };
        index.source_facts(source)
    }

    fn temporal_on_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [date, _] = args else {
            return Ok(Vec::new());
        };
        let date = constant(date)?;
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        Ok(index
            .events
            .day(index.generation, date, 10_000)
            .map_err(provider_error)?
            .records
            .into_iter()
            .map(|record| {
                compound_term(
                    "temporal_on",
                    vec![string(date), atom(record.document_id.entity.id())],
                )
            })
            .collect())
    }

    fn temporal_between_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [start, end, _] = args else {
            return Ok(Vec::new());
        };
        let start = constant(start)?;
        let end = constant(end)?;
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        let start_micros = parse_utc_micros(start).map_err(provider_error)?;
        let end_micros = parse_utc_micros(end).map_err(provider_error)?;
        Ok(index
            .events
            .range(index.generation, start_micros, end_micros, 10_000)
            .map_err(provider_error)?
            .records
            .into_iter()
            .map(|record| {
                compound_term(
                    "temporal_between",
                    vec![
                        string(start),
                        string(end),
                        atom(record.document_id.entity.id()),
                    ],
                )
            })
            .collect())
    }

    fn valid_at_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [instant, _] = args else {
            return Ok(Vec::new());
        };
        let instant = constant(instant)?;
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        let micros = parse_utc_micros(instant).map_err(provider_error)?;
        Ok(index
            .validities
            .valid_at(index.generation, micros, 10_000)
            .map_err(provider_error)?
            .records
            .into_iter()
            .map(|record| {
                compound_term(
                    "valid_at",
                    vec![string(instant), atom(record.document_id.entity.id())],
                )
            })
            .collect())
    }

    fn valid_overlaps_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [start, end, _] = args else {
            return Ok(Vec::new());
        };
        let start = constant(start)?;
        let end = constant(end)?;
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        let start_micros = parse_utc_micros(start).map_err(provider_error)?;
        let end_micros = parse_utc_micros(end).map_err(provider_error)?;
        Ok(index
            .validities
            .overlaps(index.generation, start_micros, end_micros, 10_000)
            .map_err(provider_error)?
            .records
            .into_iter()
            .map(|record| {
                compound_term(
                    "valid_overlaps",
                    vec![
                        string(start),
                        string(end),
                        atom(record.document_id.entity.id()),
                    ],
                )
            })
            .collect())
    }
}

#[derive(Clone, Copy)]
struct TemporalProvider<'a> {
    events: &'a EventTimeStore,
    validities: &'a ValidityStore,
    generation: &'a GenerationId,
}

fn compound(term: &Term) -> WamResult<Option<(&str, &[Term])>> {
    match term {
        Term::Compound { name, args } => Ok(Some((name, args))),
        Term::Atom(_) => Ok(None),
        _ => Err(WamError::Provider("expected callable term".to_string())),
    }
}

fn constant(term: &Term) -> WamResult<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value),
        _ => Err(WamError::Provider("expected bound constant".to_string())),
    }
}

fn atom(value: impl Into<String>) -> Term {
    Term::Atom(value.into())
}

fn string(value: impl Into<String>) -> Term {
    Term::String(value.into())
}

fn number(value: f32) -> Term {
    Term::Float(value as f64)
}

fn compound_term(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
