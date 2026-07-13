use std::collections::HashMap;

use crate::parser::Term;
use crate::wam::error::{WamError, WamResult};
use crate::wam::fact_provider::FactProvider;
use crate::wam::predicate::PredicateKey;
use crate::wam::runtime::WamRuntime;

use super::{TableDependencies, TableKey};

pub(super) fn evaluate_prepared_retrieval(
    runtime: &WamRuntime,
    query: &Term,
    provider: &dyn FactProvider,
    key: TableKey,
    variables: Vec<String>,
) -> WamResult<Vec<HashMap<String, Term>>> {
    let Term::Compound { args, .. } = query else {
        return Err(table_error("retrieve/4 requires a prepared call"));
    };
    if args
        .first()
        .is_none_or(|handle| matches!(handle, Term::Variable(_)))
        || args
            .get(1)
            .is_none_or(|options| matches!(options, Term::Variable(_)))
    {
        return Err(table_error(
            "retrieve/4 is tableable only with a bound prepared handle and options",
        ));
    }
    super::evaluator::begin_table(runtime, key.clone())?;
    let facts = provider.facts_for_goal(query)?;
    let rows = super::evaluator::answer_runtime(runtime, facts).query_all(query)?;
    let mut dependencies = TableDependencies::default();
    dependencies.predicates.insert(PredicateKey {
        name: "retrieve".into(),
        arity: 4,
    });
    super::evaluator::store_answers(runtime, &key, &variables, &rows, dependencies)?;
    Ok(rows)
}

fn table_error(message: &str) -> WamError {
    WamError::Provider(format!("tabling: {message}"))
}
