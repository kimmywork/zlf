use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use zlf_core::EntityRef;

use crate::{IndexDocument, IndexDocumentId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentManifest {
    pub entity: EntityRef,
    pub profile_name: String,
    pub profile_version: u32,
    pub source_version: u64,
    pub documents: Vec<IndexDocument>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentChanges {
    pub upserts: Vec<IndexDocument>,
    pub deletes: Vec<IndexDocumentId>,
}

impl DocumentManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.profile_name.is_empty() || self.profile_version == 0 {
            return Err("manifest profile identity is required".into());
        }
        let mut ids = BTreeSet::new();
        for document in &self.documents {
            if document.id.entity != self.entity || !ids.insert(document.id.clone()) {
                return Err("manifest documents must have unique matching entity IDs".into());
            }
        }
        Ok(())
    }
}

pub fn reconcile_manifest(
    previous: Option<&DocumentManifest>,
    desired: &DocumentManifest,
) -> Result<DocumentChanges, String> {
    desired.validate()?;
    if let Some(previous) = previous {
        previous.validate()?;
        if previous.entity != desired.entity
            || previous.profile_name != desired.profile_name
            || previous.profile_version != desired.profile_version
        {
            return Err("cannot reconcile different entity/profile manifests".into());
        }
        if desired.source_version < previous.source_version {
            return Err("manifest source version moved backwards".into());
        }
    }
    let old = previous
        .map(|manifest| document_map(&manifest.documents))
        .unwrap_or_default();
    let new = document_map(&desired.documents);
    let mut changes = DocumentChanges::default();
    for (id, document) in &new {
        if old.get(id) != Some(document) {
            changes.upserts.push(document.clone());
        }
    }
    for id in old.keys() {
        if !new.contains_key(id) {
            changes.deletes.push(id.clone());
        }
    }
    Ok(changes)
}

fn document_map(documents: &[IndexDocument]) -> BTreeMap<IndexDocumentId, IndexDocument> {
    documents
        .iter()
        .cloned()
        .map(|document| (document.id.clone(), document))
        .collect()
}
