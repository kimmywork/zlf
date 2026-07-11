use std::collections::HashMap;

use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_lowering::{lower_fact, FactMutation};
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
        self.apply_mutation(lower_fact(fact)?)
    }

    pub fn apply_mutation(&self, mutation: FactMutation) -> WamResult<()> {
        match mutation {
            FactMutation::EnsureNode {
                id,
                labels,
                properties,
            } => self.ensure_node_with_map(&id, &labels, properties),
            FactMutation::EnsureEdge {
                source,
                edge_type,
                target,
                properties,
            } => self.ensure_edge_with_map(&source, &edge_type, &target, properties),
            FactMutation::SetProperty { id, key, value } => self.set_property(&id, &key, value),
        }
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

    fn ensure_edge_with_map(
        &self,
        source: &str,
        edge_type: &str,
        target: &str,
        props: HashMap<String, Value>,
    ) -> WamResult<()> {
        self.ensure_node_with_map(source, &[], HashMap::new())?;
        self.ensure_node_with_map(target, &[], HashMap::new())?;
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

    fn set_property(&self, id: &str, key: &str, value: Value) -> WamResult<()> {
        self.storage
            .set_entity_property(id, key, value)
            .map(|_| ())
            .map_err(provider_error)
    }

    fn merge_properties(&self, id: &str, props: HashMap<String, Value>) -> WamResult<()> {
        if props.is_empty() {
            return Ok(());
        }
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

pub fn edge_id(source: &str, edge_type: &str, target: &str) -> String {
    format!("{source}:{edge_type}:{target}")
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
