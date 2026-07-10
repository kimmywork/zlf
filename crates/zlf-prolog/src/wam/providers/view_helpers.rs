use zlf_core::{Edge, Value};

use crate::parser::Term;

/// Build an "out edge" object term from an Edge record.
pub fn edge_out_term(edge: &Edge) -> Term {
    Term::Object(vec![
        ("id".to_string(), Term::Atom(edge.id.clone())),
        ("source".to_string(), Term::Atom(edge.source.clone())),
        ("type".to_string(), Term::Atom(edge.edge_type.clone())),
        ("target".to_string(), Term::Atom(edge.target.clone())),
        (
            "properties".to_string(),
            properties_to_object(&edge.properties),
        ),
    ])
}

/// Build an "in edge" object term from an Edge record.
pub fn edge_in_term(edge: &Edge) -> Term {
    Term::Object(vec![
        ("id".to_string(), Term::Atom(edge.id.clone())),
        ("source".to_string(), Term::Atom(edge.source.clone())),
        ("type".to_string(), Term::Atom(edge.edge_type.clone())),
        ("target".to_string(), Term::Atom(edge.target.clone())),
        (
            "properties".to_string(),
            properties_to_object(&edge.properties),
        ),
    ])
}

/// Convert a HashMap of properties to a Term::Object.
pub fn properties_to_object(props: &std::collections::HashMap<String, Value>) -> Term {
    let entries: Vec<(String, Term)> = props
        .iter()
        .map(|(key, value)| (key.clone(), value_to_term(value)))
        .collect();
    Term::Object(entries)
}

fn value_to_term(value: &Value) -> Term {
    match value {
        Value::String(s) => Term::String(s.clone()),
        Value::Number(n) => number_term(*n),
        Value::Bool(b) => Term::Atom(b.to_string()),
        Value::Null => Term::Atom("null".to_string()),
        Value::Array(items) => Term::List(items.iter().map(value_to_term).collect()),
        Value::Object(map) => Term::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), value_to_term(v)))
                .collect(),
        ),
    }
}

fn number_term(value: f64) -> Term {
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        Term::Integer(value as i64)
    } else {
        Term::Float(value)
    }
}

pub fn compound_term(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}
