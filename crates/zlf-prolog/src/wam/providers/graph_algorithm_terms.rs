use std::collections::HashMap;

use crate::parser::Term;

use super::error::WamError;
use super::view_helpers::compound_term;

pub(super) fn reconstruct_path(
    parent: &HashMap<String, String>,
    source: &str,
    target: &str,
) -> Vec<String> {
    let mut path = vec![target.to_string()];
    let mut current = target;
    while current != source {
        if let Some(prev) = parent.get(current) {
            path.push(prev.clone());
            current = prev;
        } else {
            break;
        }
    }
    path.reverse();
    path
}

pub(super) fn bound_usize(term: &Term) -> Option<usize> {
    match term {
        Term::Integer(n) if *n >= 0 => Some(*n as usize),
        Term::Float(n) if *n >= 0.0 => Some(*n as usize),
        _ => None,
    }
}

pub(super) fn number(value: f64) -> Term {
    Term::Integer(value as i64)
}

pub(super) fn shortest_path_term(source: &str, target: &str, nodes: Vec<String>) -> Term {
    compound_term(
        "shortest_path",
        vec![
            Term::Atom(source.to_string()),
            Term::Atom(target.to_string()),
            Term::List(nodes.into_iter().map(Term::Atom).collect()),
        ],
    )
}

pub(super) fn reachable_term(source: &str, target: &str, max_depth: usize, arity: usize) -> Term {
    let mut args = vec![
        Term::Atom(source.to_string()),
        Term::Atom(target.to_string()),
    ];
    if arity == 3 {
        args.push(number(max_depth as f64));
    }
    compound_term("reachable", args)
}

pub(super) fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
