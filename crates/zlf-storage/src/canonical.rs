use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde::Serialize;
use zlf_core::{Edge, Node, Result, Value, ZlfError};

use crate::NodeVersion;

#[derive(Serialize)]
struct CanonicalNode {
    id: String,
    labels: Vec<String>,
    properties: BTreeMap<String, CanonicalValue>,
    current_version: u64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct CanonicalEdge {
    id: String,
    edge_type: String,
    source: String,
    target: String,
    properties: BTreeMap<String, CanonicalValue>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct CanonicalVersion {
    version_id: u64,
    properties: BTreeMap<String, CanonicalValue>,
    valid_from: DateTime<Utc>,
    valid_to: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
enum CanonicalValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<CanonicalValue>),
    Object(BTreeMap<String, CanonicalValue>),
}

pub(crate) fn serialize_node(node: &Node) -> Result<Vec<u8>> {
    serialize(&CanonicalNode {
        id: node.id.clone(),
        labels: node.labels.clone(),
        properties: canonical_properties(&node.properties),
        current_version: node.current_version,
        created_at: node.created_at,
        updated_at: node.updated_at,
    })
}

pub(crate) fn serialize_edge(edge: &Edge) -> Result<Vec<u8>> {
    serialize(&CanonicalEdge {
        id: edge.id.clone(),
        edge_type: edge.edge_type.clone(),
        source: edge.source.clone(),
        target: edge.target.clone(),
        properties: canonical_properties(&edge.properties),
        created_at: edge.created_at,
        updated_at: edge.updated_at,
    })
}

pub(crate) fn serialize_version(version: &NodeVersion) -> Result<Vec<u8>> {
    serialize(&CanonicalVersion {
        version_id: version.version_id,
        properties: canonical_properties(&version.properties),
        valid_from: version.valid_from,
        valid_to: version.valid_to,
    })
}

fn canonical_properties(values: &HashMap<String, Value>) -> BTreeMap<String, CanonicalValue> {
    values
        .iter()
        .map(|(key, value)| (key.clone(), canonical_value(value)))
        .collect()
}

fn canonical_value(value: &Value) -> CanonicalValue {
    match value {
        Value::Null => CanonicalValue::Null,
        Value::Bool(value) => CanonicalValue::Bool(*value),
        Value::Number(value) => CanonicalValue::Number(*value),
        Value::String(value) => CanonicalValue::String(value.clone()),
        Value::Array(values) => CanonicalValue::Array(values.iter().map(canonical_value).collect()),
        Value::Object(values) => CanonicalValue::Object(canonical_properties(values)),
    }
}

fn serialize(value: &impl Serialize) -> Result<Vec<u8>> {
    bincode::serialize(value).map_err(|error| ZlfError::Serialization(error.to_string()))
}
