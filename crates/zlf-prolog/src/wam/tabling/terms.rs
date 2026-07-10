use std::collections::HashMap;

use crate::parser::{PrologRule, Term};
use crate::wam::predicate::{predicate_key, PredicateKey};

pub(crate) fn normalize_linear_recursion(rule: &PrologRule, target: &PredicateKey) -> PrologRule {
    let Some((head, prefix, prefix_name)) = linear_recursion_args(rule, target) else {
        return rule.clone();
    };
    PrologRule {
        head: rule.head.clone(),
        body: vec![
            Term::Compound {
                name: target.name.clone(),
                args: vec![head[0].clone(), prefix[1].clone()],
            },
            Term::Compound {
                name: prefix_name.to_string(),
                args: vec![prefix[1].clone(), head[1].clone()],
            },
        ],
    }
}

fn linear_recursion_args<'a>(
    rule: &'a PrologRule,
    target: &PredicateKey,
) -> Option<(&'a [Term], &'a [Term], &'a str)> {
    let Term::Compound { args: head, .. } = &rule.head else {
        return None;
    };
    let [Term::Compound { name, args: prefix }, Term::Compound {
        args: recursive, ..
    }] = rule.body.as_slice()
    else {
        return None;
    };
    let shape_matches = head.len() == 2
        && prefix.len() == 2
        && recursive.len() == 2
        && predicate_key(&rule.body[1]).as_ref() == Some(target)
        && prefix[0] == head[0]
        && recursive[1] == head[1]
        && prefix[1] == recursive[0];
    shape_matches.then_some((head, prefix, name))
}

pub(crate) fn seed_rule(rule: &PrologRule, query: &Term) -> Option<PrologRule> {
    let (
        Term::Compound {
            args: head_args, ..
        },
        Term::Compound {
            args: call_args, ..
        },
    ) = (&rule.head, query)
    else {
        return Some(rule.clone());
    };
    let bindings = seed_bindings(head_args, call_args)?;
    Some(PrologRule {
        head: substitute(&rule.head, &bindings),
        body: rule
            .body
            .iter()
            .map(|goal| substitute(goal, &bindings))
            .collect(),
    })
}

fn seed_bindings(head_args: &[Term], call_args: &[Term]) -> Option<HashMap<String, Term>> {
    let mut bindings = HashMap::new();
    for (head, call) in head_args
        .iter()
        .zip(call_args)
        .filter(|(_, call)| is_ground(call))
    {
        match head {
            Term::Variable(name) if bindings.get(name).is_some_and(|bound| bound != call) => {
                return None;
            }
            Term::Variable(name) => {
                bindings.insert(name.clone(), call.clone());
            }
            constant if constant != call => return None,
            _ => {}
        }
    }
    Some(bindings)
}

pub(crate) fn query_variables(term: &Term) -> Vec<String> {
    let mut variables = Vec::new();
    collect_variables(term, &mut variables);
    variables
}

pub(crate) fn rule_variables(rule: &PrologRule) -> Vec<String> {
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
        Term::Object(entries) => entries
            .iter()
            .for_each(|(_, value)| collect_variables(value, variables)),
        _ => {}
    }
}

pub(crate) fn substitute(term: &Term, bindings: &HashMap<String, Term>) -> Term {
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

pub(crate) fn is_ground(term: &Term) -> bool {
    match term {
        Term::Variable(_) => false,
        Term::Compound { args, .. } | Term::List(args) => args.iter().all(is_ground),
        Term::Object(entries) => entries.iter().all(|(_, value)| is_ground(value)),
        _ => true,
    }
}
