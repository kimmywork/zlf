use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;
use zlf_core::{Edge, Node, Value};
use zlf_storage::Storage;

pub struct StorageFactProvider<'a> {
    pub(crate) storage: &'a Storage,
}

impl<'a> StorageFactProvider<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }
}

impl FactProvider for StorageFactProvider<'_> {
    fn facts_for(&self, key: &PredicateKey) -> WamResult<Vec<Term>> {
        match (key.name.as_str(), key.arity) {
            ("node", 1) => self.node_facts(),
            ("label", 2) => self.label_facts(),
            ("property", 3) => self.property_facts(),
            ("edge", 3) => self.edge_facts(),
            (name, 1) => self.label_shortcut_facts(name),
            (name, 2) if name.starts_with("prop_") => self.property_shortcut_facts(name),
            (edge_type, 2) => self.edge_type_facts(edge_type),
            _ => Ok(Vec::new()),
        }
    }

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        self.facts_for_bound_goal(goal)?.map_or_else(
            || {
                super::predicate::predicate_key(goal)
                    .map_or_else(|| Ok(Vec::new()), |key| self.facts_for(&key))
            },
            Ok,
        )
    }
}

impl StorageFactProvider<'_> {
    fn node_facts(&self) -> WamResult<Vec<Term>> {
        Ok(self
            .nodes()?
            .into_iter()
            .map(|node| compound("node", vec![atom(node.id)]))
            .collect())
    }

    fn label_facts(&self) -> WamResult<Vec<Term>> {
        Ok(self.nodes()?.into_iter().flat_map(label_terms).collect())
    }

    fn property_facts(&self) -> WamResult<Vec<Term>> {
        let mut facts: Vec<_> = self
            .nodes()?
            .into_iter()
            .flat_map(node_properties)
            .collect();
        facts.extend(self.edges()?.into_iter().flat_map(edge_properties));
        Ok(facts)
    }

    fn edge_facts(&self) -> WamResult<Vec<Term>> {
        Ok(self.edges()?.into_iter().map(edge_term).collect())
    }

    fn edge_type_facts(&self, edge_type: &str) -> WamResult<Vec<Term>> {
        self.storage
            .get_edges_by_type(edge_type)
            .map(|edges| edges.into_iter().map(edge_type_term).collect())
            .map_err(|error| WamError::Provider(error.to_string()))
    }

    fn label_shortcut_facts(&self, label: &str) -> WamResult<Vec<Term>> {
        self.storage
            .get_nodes_by_label(label)
            .map(|nodes| {
                nodes
                    .into_iter()
                    .map(|node| label_shortcut_term(label, node))
                    .collect()
            })
            .map_err(|error| WamError::Provider(error.to_string()))
    }

    fn property_shortcut_facts(&self, name: &str) -> WamResult<Vec<Term>> {
        let key = name.trim_start_matches("prop_");
        let mut facts: Vec<_> = self
            .nodes()?
            .into_iter()
            .filter_map(|node| node_property_shortcut_term(key, node))
            .collect();
        facts.extend(
            self.edges()?
                .into_iter()
                .filter_map(|edge| edge_property_shortcut_term(key, edge)),
        );
        Ok(facts)
    }

    fn nodes(&self) -> WamResult<Vec<Node>> {
        self.storage
            .get_all_nodes()
            .map_err(|error| WamError::Provider(error.to_string()))
    }

    fn edges(&self) -> WamResult<Vec<Edge>> {
        self.storage
            .get_all_edges()
            .map_err(|error| WamError::Provider(error.to_string()))
    }
}

fn label_terms(node: Node) -> Vec<Term> {
    node.labels
        .into_iter()
        .map(|label| compound("label", vec![atom(node.id.clone()), atom(label)]))
        .collect()
}

fn node_properties(node: Node) -> Vec<Term> {
    node.properties
        .into_iter()
        .map(|(key, value)| property_term(node.id.clone(), key, value))
        .collect()
}

fn edge_properties(edge: Edge) -> Vec<Term> {
    edge.properties
        .into_iter()
        .map(|(key, value)| property_term(edge.id.clone(), key, value))
        .collect()
}

fn property_term(id: String, key: String, value: Value) -> Term {
    compound("property", vec![atom(id), atom(key), value_term(value)])
}

fn edge_term(edge: Edge) -> Term {
    compound(
        "edge",
        vec![atom(edge.source), atom(edge.edge_type), atom(edge.target)],
    )
}

fn edge_type_term(edge: Edge) -> Term {
    compound(edge.edge_type, vec![atom(edge.source), atom(edge.target)])
}

fn label_shortcut_term(label: &str, node: Node) -> Term {
    compound(label, vec![atom(node.id)])
}

fn node_property_shortcut_term(key: &str, node: Node) -> Option<Term> {
    node.properties.get(key).cloned().map(|value| {
        compound(
            format!("prop_{key}"),
            vec![atom(node.id), value_term(value)],
        )
    })
}

fn edge_property_shortcut_term(key: &str, edge: Edge) -> Option<Term> {
    edge.properties.get(key).cloned().map(|value| {
        compound(
            format!("prop_{key}"),
            vec![atom(edge.id), value_term(value)],
        )
    })
}

pub(crate) fn value_term(value: Value) -> Term {
    match value {
        Value::String(value) => Term::String(value),
        Value::Number(value) => number_term(value),
        Value::Bool(value) => atom(value.to_string()),
        Value::Null => atom("null"),
        Value::Array(_) => atom("array"),
        Value::Object(_) => atom("object"),
    }
}

fn number_term(value: f64) -> Term {
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        Term::Integer(value as i64)
    } else {
        Term::Float(value)
    }
}

fn atom(value: impl Into<String>) -> Term {
    Term::Atom(value.into())
}

fn compound(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}
