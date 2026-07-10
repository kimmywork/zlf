use crate::parser::Term;

use super::error::{WamError, WamResult};

const ATOM: &str = "$a:";
const STRING: &str = "$s:";
const INTEGER: &str = "$i:";
const FLOAT: &str = "$f:";

pub(crate) fn encode(term: &Term) -> WamResult<String> {
    match term {
        Term::Atom(value) => Ok(format!("{ATOM}{value}")),
        Term::String(value) => Ok(format!("{STRING}{value}")),
        Term::Integer(value) => Ok(format!("{INTEGER}{value}")),
        Term::Float(value) => Ok(format!("{FLOAT}{value}")),
        _ => Err(WamError::UnsupportedTerm("non-constant")),
    }
}

pub(crate) fn decode(value: &str) -> Term {
    if let Some(value) = value.strip_prefix(ATOM) {
        if value == "[]" {
            Term::List(Vec::new())
        } else {
            Term::Atom(value.to_string())
        }
    } else if let Some(value) = value.strip_prefix(STRING) {
        Term::String(value.to_string())
    } else if let Some(value) = value.strip_prefix(INTEGER) {
        value
            .parse::<i64>()
            .map(Term::Integer)
            .unwrap_or_else(|_| Term::Atom(value.to_string()))
    } else if let Some(value) = value.strip_prefix(FLOAT) {
        value
            .parse::<f64>()
            .map(Term::Float)
            .unwrap_or_else(|_| Term::Atom(value.to_string()))
    } else {
        Term::Atom(value.to_string())
    }
}
