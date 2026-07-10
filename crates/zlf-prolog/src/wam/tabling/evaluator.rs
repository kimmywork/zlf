use std::collections::{HashMap, HashSet};

use crate::parser::{PrologRule, Term};

use super::{TableKey, TableState};
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
    let facts = compute_fixpoint(runtime, provider, storage)?;
    let answer_runtime = answer_runtime(runtime, facts);
    let rows = answer_runtime.query_all(query)?;
    store_answers(runtime, &key, &variables, &rows)?;
    Ok(rows)
}

fn compute_fixpoint(
    runtime: &WamRuntime,
    provider: &dyn FactProvider,
    storage: Option<&Storage>,
) -> WamResult<Vec<Term>> {
    let rules = tabled_rules(runtime)?;
    validate_rules(&rules)?;
    let limits = runtime.tables.read().map_err(lock_error)?.limits;
    let mut facts = initial_tabled_facts(runtime, provider)?;
    let mut fingerprints = facts.iter().map(fingerprint).collect::<HashSet<_>>();
    for _ in 0..limits.max_iterations {
        let mut changed = false;
        for rule in &rules {
            for answer in evaluate_rule(runtime, rule, &facts, provider, storage)? {
                if fingerprints.insert(fingerprint(&answer)) {
                    if facts.len() >= limits.max_answers_per_table {
                        return Err(table_error("maximum table answers exceeded"));
                    }
                    facts.push(answer);
                    changed = true;
                }
            }
        }
        if !changed {
            return Ok(facts);
        }
    }
    Err(table_error("maximum table iterations exceeded"))
}

fn evaluate_rule(
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

fn tabled_rules(runtime: &WamRuntime) -> WamResult<Vec<PrologRule>> {
    let mut rules = runtime
        .rules
        .iter()
        .filter(|rule| predicate_key(&rule.head).is_some_and(|key| runtime.tabled.contains(&key)))
        .cloned()
        .collect::<Vec<_>>();
    rules.extend(
        runtime
            .compiled_rules
            .iter()
            .filter(|artifact| runtime.tabled.contains(&artifact.key))
            .map(|artifact| artifact.source.clone()),
    );
    if rules.is_empty() {
        Err(table_error("tabled predicate has no rules"))
    } else {
        Ok(rules)
    }
}

fn validate_rules(rules: &[PrologRule]) -> WamResult<()> {
    const UNSUPPORTED: &[&str] = &["\\+", "!", "asserta", "assertz", "retract", "retractall"];
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
    let mut tables = runtime.tables.write().map_err(lock_error)?;
    if tables.get(&key).is_none() && tables.len() >= tables.limits.max_tables {
        return Err(table_error("maximum tables exceeded"));
    }
    tables.begin(key);
    Ok(())
}

fn store_answers(
    runtime: &WamRuntime,
    key: &TableKey,
    variables: &[String],
    rows: &[HashMap<String, Term>],
) -> WamResult<()> {
    let mut tables = runtime.tables.write().map_err(lock_error)?;
    let entry = tables.begin(key.clone());
    for row in rows {
        entry.insert(
            variables
                .iter()
                .filter_map(|name| row.get(name).cloned())
                .collect(),
        );
    }
    tables.complete(key);
    Ok(())
}

fn cached_rows(
    runtime: &WamRuntime,
    key: &TableKey,
    variables: &[String],
) -> WamResult<Option<Vec<HashMap<String, Term>>>> {
    let tables = runtime.tables.read().map_err(lock_error)?;
    let Some(entry) = tables.get(key) else {
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

fn query_variables(term: &Term) -> Vec<String> {
    let mut variables = Vec::new();
    collect_variables(term, &mut variables);
    variables
}

fn rule_variables(rule: &PrologRule) -> Vec<String> {
    let mut variables = Vec::new();
    collect_variables(&rule.head, &mut variables);
    for goal in &rule.body {
        collect_variables(goal, &mut variables);
    }
    variables
}

fn collect_variables(term: &Term, variables: &mut Vec<String>) {
    match term {
        Term::Variable(name) if name != "_" && !variables.contains(name) => {
            variables.push(name.clone());
        }
        Term::Compound { args, .. } | Term::List(args) => {
            args.iter()
                .for_each(|arg| collect_variables(arg, variables));
        }
        Term::Object(entries) => {
            entries
                .iter()
                .for_each(|(_, value)| collect_variables(value, variables));
        }
        _ => {}
    }
}

fn substitute(term: &Term, bindings: &HashMap<String, Term>) -> Term {
    match term {
        Term::Variable(name) => bindings.get(name).cloned().unwrap_or_else(|| term.clone()),
        Term::Compound { name, args } => Term::Compound {
            name: name.clone(),
            args: args.iter().map(|arg| substitute(arg, bindings)).collect(),
        },
        Term::List(items) => Term::List(
            items
                .iter()
                .map(|item| substitute(item, bindings))
                .collect(),
        ),
        Term::Object(entries) => Term::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), substitute(value, bindings)))
                .collect(),
        ),
        _ => term.clone(),
    }
}

fn is_ground(term: &Term) -> bool {
    match term {
        Term::Variable(_) => false,
        Term::Compound { args, .. } | Term::List(args) => args.iter().all(is_ground),
        Term::Object(entries) => entries.iter().all(|(_, value)| is_ground(value)),
        _ => true,
    }
}

fn fingerprint(term: &Term) -> u64 {
    bincode::serialize(term)
        .unwrap_or_default()
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn lock_error(error: impl std::fmt::Display) -> WamError {
    table_error(&error.to_string())
}

fn table_error(message: &str) -> WamError {
    WamError::Provider(format!("tabling: {message}"))
}
