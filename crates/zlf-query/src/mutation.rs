use std::collections::BTreeSet;

use zlf_core::{PropertyPatch, Result, Value};
use zlf_prolog::wam::PredicateKey;
use zlf_storage::MutationReceipt;

use crate::ZlfDatabase;

impl ZlfDatabase {
    pub fn patch_node_properties(
        &self,
        id: &str,
        patch: &PropertyPatch,
    ) -> Result<MutationReceipt> {
        let receipt = self.storage.patch_node_properties(id, patch)?;
        self.finish_property_mutation(patch, &receipt)?;
        Ok(receipt)
    }

    pub fn patch_edge_properties(
        &self,
        id: &str,
        patch: &PropertyPatch,
    ) -> Result<MutationReceipt> {
        let receipt = self.storage.patch_edge_properties(id, patch)?;
        self.finish_property_mutation(patch, &receipt)?;
        Ok(receipt)
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

    pub fn get_edge_ids(&self, source: &str, edge_type: &str, target: &str) -> Result<Vec<String>> {
        self.storage.get_edge_ids(source, edge_type, target)
    }

    fn finish_property_mutation(
        &self,
        patch: &PropertyPatch,
        receipt: &MutationReceipt,
    ) -> Result<()> {
        if receipt.sequence.is_none() {
            return Ok(());
        }
        let mut predicates = vec![predicate("property", 3)];
        predicates.extend(
            patch
                .set
                .keys()
                .chain(patch.remove.iter())
                .map(|key| predicate(&format!("prop_{key}"), 2)),
        );
        self.invalidate_predicates(&predicates)?;
        self.refresh_registry()?;
        self.catch_up_indexes()
    }
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

fn predicate(name: &str, arity: usize) -> PredicateKey {
    PredicateKey {
        name: name.to_string(),
        arity,
    }
}
