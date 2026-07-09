use std::collections::HashMap;

use crate::parser::Term;

use super::error::{WamError, WamResult};
use zlf_core::{Edge, Node, Value};
use zlf_storage::Storage;

pub struct StorageFactWriter<'a> {
    pub(crate) storage: &'a Storage,
}

impl<'a> StorageFactWriter<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn apply_fact(&self, fact: &Term) -> WamResult<()> {
        let (name, args) = compound(fact)?;
        match (name, args) {
            ("node", [id]) => self.ensure_node(atom(id)?, &[]),
            ("node", [id, props]) => self.ensure_node_with_props(atom(id)?, &[], props),
            ("node", [id, labels, props]) => {
                self.ensure_node_with_props(atom(id)?, &labels_from_term(labels)?, props)
            }
            ("edge", [source, edge_type, target]) => {
                self.ensure_edge(atom(source)?, atom(edge_type)?, atom(target)?)
            }
            ("edge", [source, edge_type, target, props]) => {
                self.ensure_edge_with_props(atom(source)?, atom(edge_type)?, atom(target)?, props)
            }
            ("property", [id, key, value]) => self.set_property(atom(id)?, atom(key)?, value),
            (name, [id]) => self.ensure_node(atom(id)?, &[name.to_string()]),
            (name, [id, value]) if name.starts_with("prop_") => {
                self.set_property(atom(id)?, name.trim_start_matches("prop_"), value)
            }
            (name, [source, target, props]) => {
                self.ensure_edge_with_props(atom(source)?, name, atom(target)?, props)
            }
            (name, [source, target]) => self.ensure_edge(atom(source)?, name, atom(target)?),
            _ => Err(WamError::Provider("unsupported storage fact".to_string())),
        }
    }

    fn ensure_node(&self, id: &str, labels: &[String]) -> WamResult<()> {
        self.ensure_node_with_map(id, labels, HashMap::new())
    }

    fn ensure_node_with_props(&self, id: &str, labels: &[String], props: &Term) -> WamResult<()> {
        self.ensure_node_with_map(id, labels, properties_from_term(props)?)
    }

    fn ensure_node_with_map(
        &self,
        id: &str,
        labels: &[String],
        props: HashMap<String, Value>,
    ) -> WamResult<()> {
        if self.node_exists(id)? {
            if !labels.is_empty() {
                self.storage
                    .add_labels(id, labels)
                    .map_err(provider_error)?;
            }
            return self.merge_properties(id, props);
        }
        self.storage
            .create_node(Node::with_id(id.to_string(), labels.to_vec(), props))
            .map(|_| ())
            .map_err(provider_error)
    }

    fn ensure_edge(&self, source: &str, edge_type: &str, target: &str) -> WamResult<()> {
        self.ensure_edge_with_map(source, edge_type, target, HashMap::new())
    }

    fn ensure_edge_with_props(
        &self,
        source: &str,
        edge_type: &str,
        target: &str,
        props: &Term,
    ) -> WamResult<()> {
        self.ensure_edge_with_map(source, edge_type, target, properties_from_term(props)?)
    }

    fn ensure_edge_with_map(
        &self,
        source: &str,
        edge_type: &str,
        target: &str,
        props: HashMap<String, Value>,
    ) -> WamResult<()> {
        self.ensure_node(source, &[])?;
        self.ensure_node(target, &[])?;
        if self.edge_exists(source, edge_type, target)? {
            return Ok(());
        }
        self.storage
            .create_edge(Edge::with_id(
                edge_id(source, edge_type, target),
                edge_type.to_string(),
                source.to_string(),
                target.to_string(),
                props,
            ))
            .map(|_| ())
            .map_err(provider_error)
    }

    fn set_property(&self, id: &str, key: &str, value: &Term) -> WamResult<()> {
        self.ensure_node(id, &[])?;
        self.merge_properties(id, [(key.to_string(), value_to_storage(value)?)].into())
    }

    fn merge_properties(&self, id: &str, props: HashMap<String, Value>) -> WamResult<()> {
        let mut node = self
            .storage
            .get_node(id)
            .map_err(provider_error)?
            .ok_or_else(|| WamError::Provider(format!("missing node: {id}")))?;
        node.properties.extend(props);
        self.storage
            .update_node(id, node.properties)
            .map(|_| ())
            .map_err(provider_error)
    }

    fn node_exists(&self, id: &str) -> WamResult<bool> {
        self.storage
            .get_node(id)
            .map(|node| node.is_some())
            .map_err(provider_error)
    }

    fn edge_exists(&self, source: &str, edge_type: &str, target: &str) -> WamResult<bool> {
        self.storage
            .get_edge(&edge_id(source, edge_type, target))
            .map(|edge| edge.is_some())
            .map_err(provider_error)
    }
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

fn properties_from_term(term: &Term) -> WamResult<HashMap<String, Value>> {
    match term {
        Term::Object(entries) => entries
            .iter()
            .map(|(key, value)| Ok((key.clone(), value_to_storage(value)?)))
            .collect(),
        _ => Err(WamError::Provider("expected property object".to_string())),
    }
}

fn value_to_storage(term: &Term) -> WamResult<Value> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(Value::String(value.clone())),
        Term::Number(value) => Ok(Value::Number(*value)),
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

pub fn edge_id(source: &str, edge_type: &str, target: &str) -> String {
    format!("{source}:{edge_type}:{target}")
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
