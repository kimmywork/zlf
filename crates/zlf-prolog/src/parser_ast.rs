use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Term {
    Variable(String),
    Atom(String),
    Integer(i64),
    Float(f64),
    String(String),
    Compound { name: String, args: Vec<Term> },
    List(Vec<Term>),
    Object(Vec<(String, Term)>),
}

impl Term {
    pub fn predicate_name(&self) -> String {
        match self {
            Term::Compound { name, .. } => name.clone(),
            Term::Atom(name) => name.clone(),
            _ => String::new(),
        }
    }

    pub fn is_variable(&self) -> bool {
        matches!(self, Term::Variable(_))
    }

    pub fn is_atom(&self) -> bool {
        matches!(self, Term::Atom(_))
    }

    pub fn as_variable(&self) -> Option<&str> {
        match self {
            Term::Variable(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_atom(&self) -> Option<&str> {
        match self {
            Term::Atom(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Term::Integer(value) => Some(*value as f64),
            Term::Float(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Term::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_compound(&self) -> Option<(&str, &[Term])> {
        match self {
            Term::Compound { name, args } => Some((name, args)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fact {
    pub head: Term,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrologRule {
    pub head: Term,
    pub body: Vec<Term>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Query {
    Goal(Term),
    Goals(Vec<Term>),
    RuleDef(PrologRule),
    Directive(Term),
}
