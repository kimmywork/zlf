use std::collections::BTreeMap;

use chrono::NaiveDate;
use zlf_core::EntityRef;
use zlf_index::{
    BM25Index, EmbeddingModelProfile, ExactVectorStore, GenerationId, TemporalIndex, VectorQuery,
};

use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;

#[derive(Default)]
pub struct IndexFactProvider<'a> {
    bm25: Option<&'a BM25Index>,
    vector: Option<ExactVectorProvider<'a>>,
    temporal: Option<&'a TemporalIndex>,
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
        self.vector = Some(ExactVectorProvider {
            store,
            profile,
            generation,
        });
        self
    }

    pub fn with_temporal(mut self, temporal: &'a TemporalIndex) -> Self {
        self.temporal = Some(temporal);
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
        let date = parse_date(constant(date)?)?;
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        Ok(index
            .get_entries_for_date(date)
            .map_err(provider_error)?
            .into_iter()
            .map(|entry| {
                compound_term(
                    "temporal_on",
                    vec![string(date.to_string()), atom(entry.node_id)],
                )
            })
            .collect())
    }

    fn temporal_between_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [start, end, _] = args else {
            return Ok(Vec::new());
        };
        let start = parse_date(constant(start)?)?;
        let end = parse_date(constant(end)?)?;
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        Ok(index
            .get_entries_in_range(start, end)
            .map_err(provider_error)?
            .into_iter()
            .map(|entry| {
                compound_term(
                    "temporal_between",
                    vec![
                        string(start.to_string()),
                        string(end.to_string()),
                        atom(entry.node_id),
                    ],
                )
            })
            .collect())
    }
}

#[derive(Clone, Copy)]
struct ExactVectorProvider<'a> {
    store: &'a ExactVectorStore,
    profile: &'a EmbeddingModelProfile,
    generation: &'a GenerationId,
}

impl ExactVectorProvider<'_> {
    fn source_facts(&self, source: &str) -> WamResult<Vec<Term>> {
        Ok(self
            .source_scores(source)?
            .into_iter()
            .map(|(node, score)| {
                compound_term(
                    "vector_similar",
                    vec![atom(source), atom(node), number(score)],
                )
            })
            .collect())
    }

    fn source_scores(&self, source: &str) -> WamResult<Vec<(String, f32)>> {
        let records = self
            .store
            .records_for_entity(
                &self.generation.0,
                &self.profile.id,
                self.profile.version,
                &EntityRef::Node(source.into()),
            )
            .map_err(provider_error)?;
        let mut scores = BTreeMap::<String, f32>::new();
        for record in records {
            self.merge_record_scores(source, record.values, &mut scores)?;
        }
        let mut scores = scores.into_iter().collect::<Vec<_>>();
        scores.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        Ok(scores)
    }

    fn merge_record_scores(
        &self,
        source: &str,
        values: Vec<f32>,
        scores: &mut BTreeMap<String, f32>,
    ) -> WamResult<()> {
        let query = VectorQuery {
            generation: self.generation.clone(),
            model_profile: self.profile.id.clone(),
            model_version: self.profile.version,
            values,
            top_k: 100,
            threshold: Some(0.0),
            include_sources: Vec::new(),
            exclude_sources: Vec::new(),
            metadata: BTreeMap::new(),
        };
        for hit in self
            .store
            .search(&query, self.profile)
            .map_err(provider_error)?
        {
            let target = hit.key.document_id.entity.id();
            if target != source {
                scores
                    .entry(target.to_string())
                    .and_modify(|score| *score = score.max(hit.score))
                    .or_insert(hit.score);
            }
        }
        Ok(())
    }
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

fn parse_date(value: &str) -> WamResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| WamError::Provider(error.to_string()))
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
