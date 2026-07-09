use chrono::NaiveDate;
use zlf_index::{BM25Index, TemporalIndex, VectorIndex};

use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;

#[derive(Default)]
pub struct IndexFactProvider<'a> {
    bm25: Option<&'a BM25Index>,
    vector: Option<&'a VectorIndex>,
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

    pub fn with_vector(mut self, vector: &'a VectorIndex) -> Self {
        self.vector = Some(vector);
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
        let Some(entry) = index.get_entry(source).map_err(provider_error)? else {
            return Ok(Vec::new());
        };
        Ok(index
            .find_similar(&entry.embedding, 0.0, 100)
            .map_err(provider_error)?
            .into_iter()
            .map(|(node, score)| {
                compound_term(
                    "vector_similar",
                    vec![atom(source), atom(node), number(score)],
                )
            })
            .collect())
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
    Term::Number(value as f64)
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
