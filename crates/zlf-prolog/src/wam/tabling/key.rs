use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::parser::Term;

use super::super::predicate::{predicate_key, PredicateKey};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NormalizedTerm {
    Var(usize),
    Atom(String),
    Integer(i64),
    Float(u64),
    String(String),
    Compound(String, Vec<NormalizedTerm>),
    List(Vec<NormalizedTerm>),
    Object(Vec<(String, NormalizedTerm)>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableKey {
    pub predicate: PredicateKey,
    pub arguments: Vec<NormalizedTerm>,
}

impl TableKey {
    pub fn from_call(call: &Term) -> Option<Self> {
        let predicate = predicate_key(call)?;
        let args = match call {
            Term::Atom(_) => &[][..],
            Term::Compound { args, .. } => args,
            _ => return None,
        };
        let mut variables = HashMap::new();
        let arguments = args
            .iter()
            .map(|term| normalize(term, &mut variables))
            .collect();
        Some(Self {
            predicate,
            arguments,
        })
    }
}

fn normalize(term: &Term, variables: &mut HashMap<String, usize>) -> NormalizedTerm {
    match term {
        Term::Variable(name) => normalize_variable(name, variables),
        Term::Atom(value) => NormalizedTerm::Atom(value.clone()),
        Term::Integer(value) => NormalizedTerm::Integer(*value),
        Term::Float(value) => NormalizedTerm::Float(value.to_bits()),
        Term::String(value) => NormalizedTerm::String(value.clone()),
        Term::Compound { name, args } => NormalizedTerm::Compound(
            name.clone(),
            args.iter().map(|arg| normalize(arg, variables)).collect(),
        ),
        Term::List(items) => NormalizedTerm::List(
            items
                .iter()
                .map(|item| normalize(item, variables))
                .collect(),
        ),
        Term::Object(entries) => NormalizedTerm::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), normalize(value, variables)))
                .collect(),
        ),
    }
}

fn normalize_variable(name: &str, variables: &mut HashMap<String, usize>) -> NormalizedTerm {
    let next = variables.len();
    if name == "_" {
        variables.insert(format!("$anonymous_{next}"), next);
        NormalizedTerm::Var(next)
    } else {
        NormalizedTerm::Var(*variables.entry(name.to_string()).or_insert(next))
    }
}
