use serde::{Deserialize, Serialize};

use crate::parser::Term;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PredicateKey {
    pub name: String,
    pub arity: usize,
}

pub fn predicate_key(term: &Term) -> Option<PredicateKey> {
    match term {
        Term::Atom(name) => Some(PredicateKey {
            name: name.clone(),
            arity: 0,
        }),
        Term::Compound { name, args } => Some(PredicateKey {
            name: name.clone(),
            arity: args.len(),
        }),
        _ => None,
    }
}

pub fn compound_args(term: &Term) -> Option<&[Term]> {
    match term {
        Term::Compound { args, .. } => Some(args),
        Term::Atom(_) => Some(&[]),
        _ => None,
    }
}
