use std::collections::HashMap;

use crate::parser::{PrologRule, Term};

use super::fixpoint::run_fixpoint;
use super::scc::component;
use super::terms::{
    is_ground, normalize_linear_recursion, query_variables, rule_variables, seed_rule, substitute,
};
use super::tracing_provider::DependencyProvider;
use super::{TableDependencies, TableKey, TableState};
use crate::wam::error::{WamError, WamResult};
use crate::wam::fact_provider::FactProvider;
use crate::wam::predicate::predicate_key;
use crate::wam::runtime::WamRuntime;
use zlf_storage::Storage;

pub(crate) fn evaluate_tabled(
    runtime: &WamRuntime,
    query: &Term,
    provider: &dyn FactProvider,
    storage: Option<&Storage>,
) -> WamResult<Vec<HashMap<String, Term>>> {
    let key = TableKey::from_call(query).ok_or(WamError::ExpectedFunctor(0))?;
    let variables = query_variables(query);
    if let Some(rows) = cached_rows(runtime, &key, &variables)? {
        return Ok(rows);
    }
    begin_table(runtime, key.clone())?;
    let mut dependencies = table_dependencies(runtime, query)?;
    let tracing_provider = DependencyProvider::new(provider);
    let facts = compute_fixpoint(runtime, query, &tracing_provider, storage)?;
    dependencies.facts = tracing_provider.facts()?;
    let answer_runtime = answer_runtime(runtime, facts);
    let rows = answer_runtime.query_all(query)?;
    store_answers(runtime, &key, &variables, &rows, dependencies)?;
    Ok(rows)
}

fn table_dependencies(runtime: &WamRuntime, query: &Term) -> WamResult<TableDependencies> {
    let target = predicate_key(query).ok_or(WamError::ExpectedFunctor(0))?;
    let recursive_component = component(runtime, &target);
    let mut dependencies = TableDependencies::default();
    dependencies.predicates.insert(target);
    for rule in tabled_rules(runtime, query)? {
        dependencies
            .rules
            .insert(super::super::proof::stable_rule_id(&rule));
        for goal in &rule.body {
            let Some(key) = predicate_key(goal) else {
                continue;
            };
            if !recursive_component.contains(&key) {
                dependencies.predicates.insert(key.clone());
            }
            if runtime.tabled.contains(&key) && !recursive_component.contains(&key) {
                if let Some(table) = TableKey::from_call(goal) {
                    dependencies.tables.insert(table);
                }
            }
        }
    }
    Ok(dependencies)
}

fn compute_fixpoint(
    runtime: &WamRuntime,
    query: &Term,
    provider: &dyn FactProvider,
    storage: Option<&Storage>,
) -> WamResult<Vec<Term>> {
    let rules = tabled_rules(runtime, query)?;
    validate_rules(&rules)?;
    let limits = runtime.table_manager.limits()?;
    let target = predicate_key(query).ok_or(WamError::ExpectedFunctor(0))?;
    let recursive_component = component(runtime, &target);
    let facts = initial_tabled_facts(runtime, provider)?;
    run_fixpoint(
        runtime,
        &rules,
        &recursive_component,
        facts,
        provider,
        storage,
        limits,
    )
}

pub(crate) fn evaluate_rule(
    runtime: &WamRuntime,
    rule: &PrologRule,
    table_facts: &[Term],
    provider: &dyn FactProvider,
    storage: Option<&Storage>,
) -> WamResult<Vec<Term>> {
    let variables = rule_variables(rule);
    let query = Term::Compound {
        name: "$table_rule_answer".to_string(),
        args: variables.iter().cloned().map(Term::Variable).collect(),
    };
    let mut evaluator = answer_runtime(runtime, table_facts.to_vec());
    for fact in nested_table_facts(runtime, rule, provider, storage)? {
        evaluator.add_fact(fact);
    }
    evaluator.add_rule(PrologRule {
        head: query.clone(),
        body: rule.body.clone(),
    });
    let rows = evaluator.query_all_with_provider_and_optional_storage(&query, provider, storage)?;
    Ok(rows
        .into_iter()
        .map(|bindings| substitute(&rule.head, &bindings))
        .filter(is_ground)
        .collect())
}

fn nested_table_facts(
    runtime: &WamRuntime,
    rule: &PrologRule,
    provider: &dyn FactProvider,
    storage: Option<&Storage>,
) -> WamResult<Vec<Term>> {
    let target = predicate_key(&rule.head);
    let recursive_component = target
        .as_ref()
        .map(|key| component(runtime, key))
        .unwrap_or_default();
    let mut facts = Vec::new();
    for goal in &rule.body {
        let Some(key) = predicate_key(goal) else {
            continue;
        };
        if !runtime.tabled.contains(&key) || recursive_component.contains(&key) {
            continue;
        }
        let rows = evaluate_tabled(runtime, goal, provider, storage)?;
        for row in rows {
            facts.push(substitute(goal, &row));
        }
    }
    Ok(facts)
}

fn answer_runtime(runtime: &WamRuntime, table_facts: Vec<Term>) -> WamRuntime {
    let mut evaluator = WamRuntime::new(runtime.register_count);
    for fact in runtime.facts.iter().cloned().chain(table_facts) {
        evaluator.add_fact(fact);
    }
    for rule in runtime.rules.iter().skip(runtime.system_rule_count) {
        if predicate_key(&rule.head).is_none_or(|key| !runtime.tabled.contains(&key)) {
            evaluator.add_rule(rule.clone());
        }
    }
    for artifact in &runtime.compiled_rules {
        if !runtime.tabled.contains(&artifact.key) {
            evaluator.add_compiled_rule(artifact.clone());
        }
    }
    evaluator
}

fn tabled_rules(runtime: &WamRuntime, query: &Term) -> WamResult<Vec<PrologRule>> {
    let target = predicate_key(query).ok_or(WamError::ExpectedFunctor(0))?;
    let recursive_component = component(runtime, &target);
    let select_rule = |rule: &PrologRule| {
        let key = predicate_key(&rule.head)?;
        if !recursive_component.contains(&key) {
            return None;
        }
        if key == target {
            seed_rule(&normalize_linear_recursion(rule, &target), query)
        } else {
            Some(rule.clone())
        }
    };
    let mut rules = runtime
        .rules
        .iter()
        .filter_map(select_rule)
        .collect::<Vec<_>>();
    rules.extend(
        runtime
            .compiled_rules
            .iter()
            .filter_map(|artifact| select_rule(&artifact.source)),
    );
    if rules.is_empty() {
        Err(table_error("tabled predicate has no rules"))
    } else {
        Ok(rules)
    }
}

fn validate_rules(rules: &[PrologRule]) -> WamResult<()> {
    const UNSUPPORTED: &[&str] = &["\\+", "asserta", "assertz", "retract", "retractall"];
    if rules
        .iter()
        .flat_map(|rule| &rule.body)
        .any(|goal| predicate_key(goal).is_some_and(|key| UNSUPPORTED.contains(&key.name.as_str())))
    {
        Err(table_error(
            "unsupported control or mutation in tabled predicate",
        ))
    } else {
        Ok(())
    }
}

fn initial_tabled_facts(runtime: &WamRuntime, provider: &dyn FactProvider) -> WamResult<Vec<Term>> {
    let mut facts = runtime
        .facts
        .iter()
        .filter(|fact| predicate_key(fact).is_some_and(|key| runtime.tabled.contains(&key)))
        .cloned()
        .collect::<Vec<_>>();
    for key in &runtime.tabled {
        facts.extend(provider.facts_for(key)?);
    }
    Ok(facts)
}

fn begin_table(runtime: &WamRuntime, key: TableKey) -> WamResult<()> {
    runtime.table_manager.begin(key)
}

fn store_answers(
    runtime: &WamRuntime,
    key: &TableKey,
    variables: &[String],
    rows: &[HashMap<String, Term>],
    dependencies: TableDependencies,
) -> WamResult<()> {
    runtime.table_manager.complete(
        key,
        rows.iter()
            .map(|row| {
                variables
                    .iter()
                    .filter_map(|name| row.get(name).cloned())
                    .collect()
            })
            .collect(),
        dependencies,
    )
}

fn cached_rows(
    runtime: &WamRuntime,
    key: &TableKey,
    variables: &[String],
) -> WamResult<Option<Vec<HashMap<String, Term>>>> {
    let Some(entry) = runtime.table_manager.lookup(key)? else {
        return Ok(None);
    };
    if entry.state != TableState::Complete {
        return Ok(None);
    }
    Ok(Some(
        entry
            .answers
            .iter()
            .map(|answer| {
                variables
                    .iter()
                    .cloned()
                    .zip(answer.values.clone())
                    .collect()
            })
            .collect(),
    ))
}

fn table_error(message: &str) -> WamError {
    WamError::Provider(format!("tabling: {message}"))
}
