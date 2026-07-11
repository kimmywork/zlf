use std::collections::BTreeSet;

use zlf_core::{EntityRef, PropertyPatch, Result, Value, ZlfError};

use crate::{EntityResolution, MutationReceipt, Storage};

impl Storage {
    pub fn resolve_entity(&self, id: &str) -> Result<EntityResolution> {
        match (self.get_node(id)?.is_some(), self.get_edge(id)?.is_some()) {
            (false, false) => Ok(EntityResolution::Missing),
            (true, false) => Ok(EntityResolution::Node),
            (false, true) => Ok(EntityResolution::Edge),
            (true, true) => Ok(EntityResolution::Ambiguous),
        }
    }

    pub fn get_edge_ids(&self, source: &str, edge_type: &str, target: &str) -> Result<Vec<String>> {
        let mut ids = self
            .get_outgoing_edges(source, Some(edge_type))?
            .into_iter()
            .filter(|edge| edge.target == target)
            .map(|edge| edge.id)
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    pub fn patch_node_properties(
        &self,
        id: &str,
        patch: &PropertyPatch,
    ) -> Result<MutationReceipt> {
        patch.validate()?;
        let _guard = self.write_guard()?;
        let mut node = self
            .get_node(id)?
            .ok_or_else(|| ZlfError::NodeNotFound(id.to_string()))?;
        let old = node.clone();
        let changed = apply_patch(&mut node.properties, patch);
        if changed.is_empty() {
            return Ok(MutationReceipt::default());
        }
        node.increment_version();
        let sequence = self.commit_node_upsert(Some(&old), &node, changed)?;
        Ok(receipt(EntityRef::Node(id.to_string()), sequence))
    }

    pub fn patch_edge_properties(
        &self,
        id: &str,
        patch: &PropertyPatch,
    ) -> Result<MutationReceipt> {
        patch.validate()?;
        let _guard = self.write_guard()?;
        let mut edge = self
            .get_edge(id)?
            .ok_or_else(|| ZlfError::EdgeNotFound(id.to_string()))?;
        let old = edge.clone();
        let changed = apply_patch(&mut edge.properties, patch);
        if changed.is_empty() {
            return Ok(MutationReceipt::default());
        }
        edge.updated_at = chrono::Utc::now();
        let sequence = self.commit_edge_upsert(Some(&old), &edge, changed)?;
        Ok(receipt(EntityRef::Edge(id.to_string()), sequence))
    }

    pub fn set_node_property(&self, id: &str, key: &str, value: Value) -> Result<MutationReceipt> {
        self.patch_node_properties(id, &set_patch(key, value))
    }

    pub fn remove_node_property(&self, id: &str, key: &str) -> Result<MutationReceipt> {
        self.patch_node_properties(id, &remove_patch(key))
    }

    pub fn set_edge_property(&self, id: &str, key: &str, value: Value) -> Result<MutationReceipt> {
        self.patch_edge_properties(id, &set_patch(key, value))
    }

    pub fn remove_edge_property(&self, id: &str, key: &str) -> Result<MutationReceipt> {
        self.patch_edge_properties(id, &remove_patch(key))
    }

    pub fn set_entity_property(
        &self,
        id: &str,
        key: &str,
        value: Value,
    ) -> Result<MutationReceipt> {
        match self.resolve_entity(id)? {
            EntityResolution::Node => self.set_node_property(id, key, value),
            EntityResolution::Edge => self.set_edge_property(id, key, value),
            EntityResolution::Missing => Err(ZlfError::EntityNotFound(id.to_string())),
            EntityResolution::Ambiguous => Err(ZlfError::AmbiguousEntity(id.to_string())),
        }
    }

    pub fn remove_entity_property(&self, id: &str, key: &str) -> Result<MutationReceipt> {
        match self.resolve_entity(id)? {
            EntityResolution::Node => self.remove_node_property(id, key),
            EntityResolution::Edge => self.remove_edge_property(id, key),
            EntityResolution::Missing => Err(ZlfError::EntityNotFound(id.to_string())),
            EntityResolution::Ambiguous => Err(ZlfError::AmbiguousEntity(id.to_string())),
        }
    }
}

fn apply_patch(
    properties: &mut std::collections::HashMap<String, Value>,
    patch: &PropertyPatch,
) -> BTreeSet<String> {
    let mut changed = BTreeSet::new();
    for key in &patch.remove {
        if properties.remove(key).is_some() {
            changed.insert(key.clone());
        }
    }
    for (key, value) in &patch.set {
        if properties.get(key) != Some(value) {
            properties.insert(key.clone(), value.clone());
            changed.insert(key.clone());
        }
    }
    changed
}

fn set_patch(key: &str, value: Value) -> PropertyPatch {
    PropertyPatch {
        set: [(key.to_string(), value)].into(),
        remove: BTreeSet::new(),
    }
}

fn remove_patch(key: &str) -> PropertyPatch {
    PropertyPatch {
        set: Default::default(),
        remove: BTreeSet::from([key.to_string()]),
    }
}

fn receipt(entity: EntityRef, sequence: u64) -> MutationReceipt {
    MutationReceipt {
        sequence: Some(sequence),
        entity_versions: vec![(entity, sequence)],
    }
}
