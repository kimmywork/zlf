use zlf_prolog::wam::{FactProvider, PredicateKey, WamError, WamResult};
use zlf_prolog::Term;

use crate::{PreparedRetrievalHandle, RetrievalExecutionResult, ZlfDatabase};

pub(crate) struct PreparedRetrievalProvider<'a> {
    database: &'a ZlfDatabase,
}

impl<'a> PreparedRetrievalProvider<'a> {
    pub(crate) fn new(database: &'a ZlfDatabase) -> Self {
        Self { database }
    }
}

impl FactProvider for PreparedRetrievalProvider<'_> {
    fn facts_for(&self, _key: &PredicateKey) -> WamResult<Vec<Term>> {
        Ok(Vec::new())
    }

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        let Term::Compound { name, args } = goal else {
            return Ok(Vec::new());
        };
        if name != "retrieve" {
            return Ok(Vec::new());
        }
        let [handle, options, entity, _] = args.as_slice() else {
            return Ok(Vec::new());
        };
        let handle = constant(handle)?;
        validate_options(options)?;
        let bound_entity = bound_entity(entity);
        let result = self
            .database
            .execute_prepared_retrieval_for_entity(
                &PreparedRetrievalHandle(handle.into()),
                bound_entity.as_ref(),
            )
            .map_err(|error| WamError::Provider(error.to_string()))?;
        Ok(result
            .hits
            .iter()
            .map(|hit| retrieval_fact(handle, options, hit, &result))
            .collect())
    }
}

fn retrieval_fact(
    handle: &str,
    options: &Term,
    hit: &zlf_index::RetrievalHit,
    result: &RetrievalExecutionResult,
) -> Term {
    let details = retrieval_details(hit, result);
    Term::Compound {
        name: "retrieve".into(),
        args: vec![
            Term::String(handle.into()),
            options.clone(),
            Term::Atom(hit.document_id.entity.id().into()),
            details,
        ],
    }
}

fn retrieval_details(hit: &zlf_index::RetrievalHit, result: &RetrievalExecutionResult) -> Term {
    let mut entries = vec![
        ("mode".into(), Term::Atom(result.metadata.mode.clone())),
        (
            "strategy".into(),
            Term::Atom(result.metadata.strategy.clone()),
        ),
        ("field".into(), Term::String(hit.document_id.field.clone())),
        (
            "chunk".into(),
            Term::String(hit.document_id.chunk_id.clone()),
        ),
        ("fused_rank".into(), integer(hit.fused_rank)),
        ("fused_score".into(), Term::Float(hit.fused_score)),
        (
            "candidate_budget_exhausted".into(),
            atom_bool(result.metadata.candidate_budget_exhausted),
        ),
        (
            "exact_filtered_top_k".into(),
            atom_bool(result.metadata.exact_filtered_top_k),
        ),
    ];
    entries.extend(score_details("lexical", hit.lexical.as_ref()));
    entries.extend(score_details("vector", hit.vector.as_ref()));
    Term::Object(entries)
}

fn score_details(prefix: &str, score: Option<&zlf_index::RetrieverScore>) -> Vec<(String, Term)> {
    vec![
        (format!("{prefix}_rank"), optional_rank(score)),
        (format!("{prefix}_score"), optional_score(score)),
        (format!("{prefix}_generation"), optional_generation(score)),
        (format!("{prefix}_watermark"), optional_watermark(score)),
    ]
}

fn bound_entity(term: &Term) -> Option<zlf_core::EntityRef> {
    match term {
        Term::Atom(id) | Term::String(id) => Some(zlf_core::EntityRef::Node(id.clone())),
        _ => None,
    }
}

fn constant(term: &Term) -> WamResult<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value),
        _ => Err(WamError::Provider(
            "retrieve/4 requires a bound prepared handle".into(),
        )),
    }
}

fn validate_options(options: &Term) -> WamResult<()> {
    if matches!(options, Term::Object(_) | Term::Atom(_)) {
        Ok(())
    } else {
        Err(WamError::Provider(
            "retrieve/4 options must be a bound object or atom".into(),
        ))
    }
}

fn optional_rank(score: Option<&zlf_index::RetrieverScore>) -> Term {
    score.map_or(Term::Atom("none".into()), |score| integer(score.rank))
}

fn optional_score(score: Option<&zlf_index::RetrieverScore>) -> Term {
    score.map_or(Term::Atom("none".into()), |score| {
        Term::Float(f64::from(score.score))
    })
}

fn optional_generation(score: Option<&zlf_index::RetrieverScore>) -> Term {
    score.map_or(Term::Atom("none".into()), |score| {
        Term::String(score.generation.0.clone())
    })
}

fn optional_watermark(score: Option<&zlf_index::RetrieverScore>) -> Term {
    score.map_or(Term::Atom("none".into()), |score| {
        Term::Integer(i64::try_from(score.watermark).unwrap_or(i64::MAX))
    })
}

fn integer(value: usize) -> Term {
    Term::Integer(i64::try_from(value).unwrap_or(i64::MAX))
}

fn atom_bool(value: bool) -> Term {
    Term::Atom(value.to_string())
}
