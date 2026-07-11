use std::collections::HashMap;

use crate::parser::Term;

use super::error::{WamError, WamResult};
use zlf_core::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum FactMutation {
    EnsureNode {
        id: String,
        labels: Vec<String>,
        properties: HashMap<String, Value>,
    },
    EnsureEdge {
        source: String,
        edge_type: String,
        target: String,
        properties: HashMap<String, Value>,
    },
    SetProperty {
        id: String,
        key: String,
        value: Value,
    },
}

pub fn lower_fact(fact: &Term) -> WamResult<FactMutation> {
    let (name, args) = compound(fact)?;
    match (name, args) {
        ("node", [id]) => node(id, Vec::new(), HashMap::new()),
        ("node", [id, props]) => node(id, Vec::new(), properties(props)?),
        ("node", [id, labels, props]) => node(id, labels_from_term(labels)?, properties(props)?),
        ("edge", [source, edge_type, target]) => edge(source, atom(edge_type)?, target, None),
        ("edge", [source, edge_type, target, props]) => {
            edge(source, atom(edge_type)?, target, Some(props))
        }
        ("property", [id, key, value]) => property(id, atom(key)?, value),
        (name, [id]) => node(id, vec![name.to_string()], HashMap::new()),
        (name, [id, value]) if name.starts_with("prop_") => {
            property(id, name.trim_start_matches("prop_"), value)
        }
        (name, [source, target]) => edge(source, name, target, None),
        (name, [source, target, props]) => edge(source, name, target, Some(props)),
        _ => Err(WamError::Provider("unsupported storage fact".to_string())),
    }
}

fn node(
    id: &Term,
    labels: Vec<String>,
    properties: HashMap<String, Value>,
) -> WamResult<FactMutation> {
    Ok(FactMutation::EnsureNode {
        id: atom(id)?.to_string(),
        labels,
        properties,
    })
}

fn edge(
    source: &Term,
    edge_type: &str,
    target: &Term,
    props: Option<&Term>,
) -> WamResult<FactMutation> {
    Ok(FactMutation::EnsureEdge {
        source: atom(source)?.to_string(),
        edge_type: edge_type.to_string(),
        target: atom(target)?.to_string(),
        properties: props.map(properties).transpose()?.unwrap_or_default(),
    })
}

fn property(id: &Term, key: &str, value: &Term) -> WamResult<FactMutation> {
    Ok(FactMutation::SetProperty {
        id: atom(id)?.to_string(),
        key: key.to_string(),
        value: value_to_storage(value)?,
    })
}

fn compound(term: &Term) -> WamResult<(&str, &[Term])> {
    match term {
        Term::Compound { name, args } => Ok((name, args)),
        Term::Atom(name) => Ok((name, &[])),
        _ => Err(WamError::Provider("expected fact term".to_string())),
    }
}

fn atom(term: &Term) -> WamResult<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value),
        _ => Err(WamError::Provider("expected atom".to_string())),
    }
}

fn labels_from_term(term: &Term) -> WamResult<Vec<String>> {
    match term {
        Term::List(items) => items
            .iter()
            .map(|item| atom(item).map(str::to_string))
            .collect(),
        _ => Err(WamError::Provider("expected label list".to_string())),
    }
}

fn properties(term: &Term) -> WamResult<HashMap<String, Value>> {
    match term {
        Term::Object(entries) => entries
            .iter()
            .map(|(key, value)| Ok((key.clone(), value_to_storage(value)?)))
            .collect(),
        _ => Err(WamError::Provider("expected property object".to_string())),
    }
}

pub(crate) fn value_to_storage(term: &Term) -> WamResult<Value> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(Value::String(value.clone())),
        Term::Integer(value) => Ok(Value::Number(*value as f64)),
        Term::Float(value) => Ok(Value::Number(*value)),
        Term::List(items) => items
            .iter()
            .map(value_to_storage)
            .collect::<WamResult<Vec<_>>>()
            .map(Value::Array),
        Term::Object(entries) => entries
            .iter()
            .map(|(key, value)| Ok((key.clone(), value_to_storage(value)?)))
            .collect::<WamResult<HashMap<_, _>>>()
            .map(Value::Object),
        _ => Err(WamError::Provider("unsupported property value".to_string())),
    }
}
