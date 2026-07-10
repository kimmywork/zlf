use rocksdb::WriteBatch;
use serde::{Deserialize, Serialize};
use zlf_core::{Edge, Node, Result, Value, ZlfError};

use crate::canonical::{serialize_edge, serialize_node, serialize_version};
use crate::{NodeVersion, Storage};

pub const STORAGE_KEY_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageRecord {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageRecordPlan {
    pub records: Vec<StorageRecord>,
}

impl StorageRecordPlan {
    pub fn extend(&mut self, other: Self) {
        self.records.extend(other.records);
    }
}

impl Storage {
    pub fn compile_node_records(node: &Node) -> Result<StorageRecordPlan> {
        let mut records = vec![encoded_bytes(
            format!("node:{}", node.id),
            serialize_node(node)?,
        )];
        let version = NodeVersion {
            version_id: node.current_version,
            properties: node.properties.clone(),
            valid_from: node.updated_at,
            valid_to: None,
        };
        records.push(encoded_bytes(
            format!("ver:{}:{}", node.id, node.current_version),
            serialize_version(&version)?,
        ));
        records.extend(node.labels.iter().flat_map(|label| {
            [
                index_record(format!("idx:label:{label}:{}", node.id), &[]),
                metadata_record("label", label),
            ]
        }));
        records.extend(
            node.properties
                .keys()
                .map(|key| metadata_record("property", key)),
        );
        records.extend(property_index_records(node)?);
        records.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(StorageRecordPlan { records })
    }

    pub fn compile_edge_records(edge: &Edge) -> Result<StorageRecordPlan> {
        let records = vec![
            encoded_bytes(format!("edge:{}", edge.id), serialize_edge(edge)?),
            index_record(format!("idx:edge_type:{}:{}", edge.edge_type, edge.id), &[]),
            metadata_record("edge_type", &edge.edge_type),
            index_record(
                format!(
                    "idx:edge_out:{}:{}:{}",
                    edge.source, edge.edge_type, edge.target
                ),
                edge.id.as_bytes(),
            ),
            index_record(
                format!(
                    "idx:edge_in:{}:{}:{}",
                    edge.target, edge.edge_type, edge.source
                ),
                edge.id.as_bytes(),
            ),
        ];
        Ok(StorageRecordPlan { records })
    }

    pub fn write_record_plans<'a>(
        &self,
        plans: impl IntoIterator<Item = &'a StorageRecordPlan>,
    ) -> Result<usize> {
        let mut batch = WriteBatch::default();
        let mut count = 0;
        for record in plans.into_iter().flat_map(|plan| &plan.records) {
            batch.put(&record.key, &record.value);
            count += 1;
        }
        self.db
            .write(batch)
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        Ok(count)
    }
}

pub(crate) fn property_index_key(key: &str, value: &Value) -> Result<String> {
    let encoded_key = hex(key.as_bytes());
    let encoded_value = bincode::serialize(value)
        .map(hex)
        .map_err(|error| ZlfError::Serialization(error.to_string()))?;
    Ok(format!("idx:property:{encoded_key}:{encoded_value}:"))
}

fn property_index_records(node: &Node) -> Result<Vec<StorageRecord>> {
    node.properties
        .iter()
        .filter(|(_, value)| is_scalar(value))
        .map(|(key, value)| {
            Ok(index_record(
                format!("{}{}", property_index_key(key, value)?, node.id),
                &[],
            ))
        })
        .collect()
}

fn is_scalar(value: &Value) -> bool {
    !matches!(value, Value::Array(_) | Value::Object(_))
}

fn encoded_bytes(key: String, value: Vec<u8>) -> StorageRecord {
    StorageRecord {
        key: key.into_bytes(),
        value,
    }
}

pub(crate) fn predicate_metadata_key(kind: &str, value: &str) -> String {
    format!("meta:predicate:{kind}:{}", hex(value))
}

fn metadata_record(kind: &str, value: &str) -> StorageRecord {
    index_record(predicate_metadata_key(kind, value), value.as_bytes())
}

fn index_record(key: String, value: &[u8]) -> StorageRecord {
    StorageRecord {
        key: key.into_bytes(),
        value: value.to_vec(),
    }
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
