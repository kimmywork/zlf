use std::collections::BTreeSet;

use chrono::Utc;
use rocksdb::WriteBatch;
use zlf_core::{Edge, EntityRef, Node, Result, ZlfError};

use crate::bulk::{property_index_key, StorageRecordPlan};
use crate::outbox::{entity_state_key, outbox_key, NEXT_SEQUENCE_KEY};
use crate::{EntityState, MutationEvent, MutationKind, MutationSequence, Storage};

impl Storage {
    pub(crate) fn commit_node_upsert(
        &self,
        old: Option<&Node>,
        node: &Node,
        changed_fields: BTreeSet<String>,
    ) -> Result<MutationSequence> {
        let mut batch = WriteBatch::default();
        if let Some(old) = old {
            delete_node_indexes(&mut batch, old)?;
        }
        put_plan(&mut batch, &Self::compile_node_records(node)?);
        let sequence = self.next_mutation_sequence()?;
        self.append_event_at(
            &mut batch,
            sequence,
            EntityRef::Node(node.id.clone()),
            MutationKind::Upsert { changed_fields },
        )?;
        self.db.write(batch).map_err(internal)?;
        Ok(sequence)
    }
    pub(crate) fn commit_edge_upsert(
        &self,
        old: Option<&Edge>,
        edge: &Edge,
        changed_fields: BTreeSet<String>,
    ) -> Result<MutationSequence> {
        let mut batch = WriteBatch::default();
        if let Some(old) = old {
            delete_edge_records(&mut batch, old);
        }
        put_plan(&mut batch, &Self::compile_edge_records(edge)?);
        let sequence = self.next_mutation_sequence()?;
        self.append_event_at(
            &mut batch,
            sequence,
            EntityRef::Edge(edge.id.clone()),
            MutationKind::Upsert { changed_fields },
        )?;
        self.db.write(batch).map_err(internal)?;
        Ok(sequence)
    }

    pub(crate) fn commit_edge_delete(&self, edge: &Edge) -> Result<MutationSequence> {
        let mut batch = WriteBatch::default();
        delete_edge_records(&mut batch, edge);
        let sequence = self.next_mutation_sequence()?;
        self.append_event_at(
            &mut batch,
            sequence,
            EntityRef::Edge(edge.id.clone()),
            MutationKind::Delete,
        )?;
        self.db.write(batch).map_err(internal)?;
        Ok(sequence)
    }

    pub(crate) fn commit_node_cascade_delete(
        &self,
        node: &Node,
        edges: &[Edge],
    ) -> Result<Vec<MutationSequence>> {
        let mut batch = WriteBatch::default();
        let mut sequences = Vec::with_capacity(edges.len() + 1);
        let mut sequence = self.latest_mutation_sequence()?;
        for edge in edges {
            sequence = sequence
                .checked_add(1)
                .ok_or_else(|| ZlfError::Internal("mutation sequence exhausted".into()))?;
            delete_edge_records(&mut batch, edge);
            self.append_event_at(
                &mut batch,
                sequence,
                EntityRef::Edge(edge.id.clone()),
                MutationKind::Delete,
            )?;
            sequences.push(sequence);
        }
        sequence = sequence
            .checked_add(1)
            .ok_or_else(|| ZlfError::Internal("mutation sequence exhausted".into()))?;
        self.delete_node_records(&mut batch, node)?;
        self.append_event_at(
            &mut batch,
            sequence,
            EntityRef::Node(node.id.clone()),
            MutationKind::Delete,
        )?;
        sequences.push(sequence);
        self.db.write(batch).map_err(internal)?;
        Ok(sequences)
    }

    fn delete_node_records(&self, batch: &mut WriteBatch, node: &Node) -> Result<()> {
        delete_node_indexes(batch, node)?;
        batch.delete(format!("node:{}", node.id));
        for (key, _) in self.scan_prefix(&format!("ver:{}:", node.id))? {
            batch.delete(key);
        }
        Ok(())
    }

    fn next_mutation_sequence(&self) -> Result<MutationSequence> {
        self.latest_mutation_sequence()?
            .checked_add(1)
            .ok_or_else(|| ZlfError::Internal("mutation sequence exhausted".into()))
    }

    fn append_event_at(
        &self,
        batch: &mut WriteBatch,
        sequence: MutationSequence,
        entity: EntityRef,
        kind: MutationKind,
    ) -> Result<()> {
        let state = EntityState {
            entity: entity.clone(),
            source_version: sequence,
            deleted: matches!(kind, MutationKind::Delete),
        };
        let event = MutationEvent {
            schema_version: crate::MUTATION_EVENT_SCHEMA_VERSION,
            sequence,
            entity: Some(entity.clone()),
            source_version: sequence,
            kind,
            occurred_at: Utc::now(),
        };
        batch.put(NEXT_SEQUENCE_KEY, sequence.to_be_bytes());
        batch.put(entity_state_key(&entity), serialize(&state)?);
        batch.put(outbox_key(sequence), serialize(&event)?);
        Ok(())
    }
}

fn put_plan(batch: &mut WriteBatch, plan: &StorageRecordPlan) {
    for record in &plan.records {
        batch.put(&record.key, &record.value);
    }
}

fn delete_node_indexes(batch: &mut WriteBatch, node: &Node) -> Result<()> {
    for label in &node.labels {
        batch.delete(format!("idx:label:{label}:{}", node.id));
    }
    for (key, value) in &node.properties {
        if !matches!(
            value,
            zlf_core::Value::Array(_) | zlf_core::Value::Object(_)
        ) {
            batch.delete(format!("{}{}", property_index_key(key, value)?, node.id));
        }
    }
    Ok(())
}

fn delete_edge_records(batch: &mut WriteBatch, edge: &Edge) {
    batch.delete(format!("edge:{}", edge.id));
    batch.delete(format!("idx:edge_type:{}:{}", edge.edge_type, edge.id));
    batch.delete(format!(
        "idx:edge_out:{}:{}:{}",
        edge.source, edge.edge_type, edge.target
    ));
    batch.delete(format!(
        "idx:edge_in:{}:{}:{}",
        edge.target, edge.edge_type, edge.source
    ));
}

fn serialize(value: &impl serde::Serialize) -> Result<Vec<u8>> {
    bincode::serialize(value).map_err(serialization)
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
