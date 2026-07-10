use std::collections::HashSet;

use crate::parser::{PrologRule, Term};
use crate::wam::error::{WamError, WamResult};
use crate::wam::predicate::{predicate_key, PredicateKey};

pub(crate) fn insert_new_answers(
    facts: &mut Vec<Term>,
    fingerprints: &mut HashSet<u64>,
    answers: Vec<Term>,
    maximum: usize,
) -> WamResult<Vec<Term>> {
    let mut inserted = Vec::new();
    for answer in answers {
        if !fingerprints.insert(fingerprint(&answer)) {
            continue;
        }
        if facts.len() >= maximum {
            return Err(WamError::Provider(
                "tabling: maximum table answers exceeded".to_string(),
            ));
        }
        facts.push(answer.clone());
        inserted.push(answer);
    }
    Ok(inserted)
}

pub(crate) fn is_recursive(rule: &PrologRule, component: &HashSet<PredicateKey>) -> bool {
    rule.body
        .iter()
        .filter_map(predicate_key)
        .any(|key| component.contains(&key))
}

pub(crate) fn rule_variants(
    rule: &PrologRule,
    component: &HashSet<PredicateKey>,
) -> Vec<PrologRule> {
    rule.body
        .iter()
        .enumerate()
        .filter(|(_, goal)| predicate_key(goal).is_some_and(|key| component.contains(&key)))
        .map(|(delta_index, _)| PrologRule {
            head: rule.head.clone(),
            body: rule
                .body
                .iter()
                .enumerate()
                .map(|(index, goal)| {
                    if index == delta_index {
                        renamed_delta(goal)
                    } else {
                        goal.clone()
                    }
                })
                .collect(),
        })
        .collect()
}

pub(crate) fn renamed_facts(facts: &[Term], component: &HashSet<PredicateKey>) -> Vec<Term> {
    facts
        .iter()
        .filter(|fact| predicate_key(fact).is_some_and(|key| component.contains(&key)))
        .map(renamed_delta)
        .collect()
}

fn renamed_delta(term: &Term) -> Term {
    match term {
        Term::Atom(name) => Term::Atom(delta_name(name)),
        Term::Compound { name, args } => Term::Compound {
            name: delta_name(name),
            args: args.clone(),
        },
        _ => term.clone(),
    }
}

fn fingerprint(term: &Term) -> u64 {
    bincode::serialize(term)
        .unwrap_or_default()
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn delta_name(name: &str) -> String {
    format!("$table_delta:{name}")
}
