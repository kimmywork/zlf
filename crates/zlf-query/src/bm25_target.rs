use std::collections::{BTreeMap, BTreeSet};

use zlf_core::{EntityRef, Result};
use zlf_index::{
    BM25Index, DocumentChanges, DocumentManifest, IndexDocumentId, IndexProfileArtifact,
};
use zlf_storage::{MutationEvent, MutationKind, Storage};

use crate::coordinator::{IndexTarget, TargetApplyError};
use crate::fake_documents::{active_profiles, matching_fields, profile_documents};
use crate::IndexManifestStore;

pub struct Bm25IndexTarget<'a> {
    index: &'a BM25Index,
    target: String,
}

impl<'a> Bm25IndexTarget<'a> {
    pub fn new(index: &'a BM25Index, target: &str) -> Self {
        Self {
            index,
            target: target.to_string(),
        }
    }

    fn apply_entity(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
        delete: bool,
    ) -> Result<()> {
        if delete {
            return self.delete_entity(storage, entity, source_version);
        }
        for profile in active_profiles(storage)? {
            self.apply_profile(storage, entity, source_version, profile)?;
        }
        Ok(())
    }

    fn apply_profile(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
        profile: zlf_index::IndexProfileArtifact,
    ) -> Result<()> {
        let Some(fields) = matching_fields(storage, entity, &profile)? else {
            return Ok(());
        };
        let mut documents = profile_documents(entity, source_version, &profile, &fields)?;
        documents.retain(|document| {
            profile
                .fields
                .get(&document.id.field)
                .is_some_and(|options| options.bm25.is_some())
        });
        let store = IndexManifestStore::new(storage, &self.target);
        self.retire_profile_versions(
            &store,
            entity,
            source_version,
            &profile.name,
            profile.version,
        )?;
        let manifest = DocumentManifest {
            entity: entity.clone(),
            profile_name: profile.name,
            profile_version: profile.version,
            source_version,
            documents,
        };
        let changes = store.changes(&manifest)?;
        self.apply_changes(changes)?;
        store.save(&manifest)
    }

    fn retire_profile_versions(
        &self,
        store: &IndexManifestStore<'_>,
        entity: &EntityRef,
        source_version: u64,
        name: &str,
        version: u32,
    ) -> Result<()> {
        for previous in store
            .list_for_entity(entity)?
            .into_iter()
            .filter(|manifest| manifest.profile_name == name && manifest.profile_version != version)
        {
            let empty = DocumentManifest {
                entity: entity.clone(),
                profile_name: previous.profile_name,
                profile_version: previous.profile_version,
                source_version,
                documents: Vec::new(),
            };
            let changes = store.changes(&empty)?;
            self.apply_changes(changes)?;
            store.save(&empty)?;
        }
        Ok(())
    }

    fn delete_entity(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
    ) -> Result<()> {
        let store = IndexManifestStore::new(storage, &self.target);
        for previous in store.list_for_entity(entity)? {
            let empty = DocumentManifest {
                entity: entity.clone(),
                profile_name: previous.profile_name,
                profile_version: previous.profile_version,
                source_version,
                documents: Vec::new(),
            };
            let changes = store.changes(&empty)?;
            self.apply_changes(changes)?;
            store.save(&empty)?;
        }
        Ok(())
    }

    pub(crate) fn rebuild(&self, storage: &Storage) -> Result<()> {
        let profiles = active_profiles(storage)?;
        let store = IndexManifestStore::new(storage, &self.target);
        let mut manifests = Vec::new();
        for node in storage.get_all_nodes()? {
            let entity = EntityRef::Node(node.id);
            let version = storage
                .get_entity_state(&entity)?
                .map_or(0, |state| state.source_version);
            manifests.extend(self.rebuild_manifests(storage, &store, &entity, version, &profiles)?);
        }
        for edge in storage.get_all_edges()? {
            let entity = EntityRef::Edge(edge.id);
            let version = storage
                .get_entity_state(&entity)?
                .map_or(0, |state| state.source_version);
            manifests.extend(self.rebuild_manifests(storage, &store, &entity, version, &profiles)?);
        }
        let changes = merge_changes(
            manifests
                .iter()
                .map(|manifest| store.changes(manifest))
                .collect::<Result<Vec<_>>>()?,
        );
        self.apply_changes(changes)?;
        for manifest in manifests {
            store.save(&manifest)?;
        }
        Ok(())
    }

    fn rebuild_manifests(
        &self,
        storage: &Storage,
        store: &IndexManifestStore<'_>,
        entity: &EntityRef,
        source_version: u64,
        profiles: &[IndexProfileArtifact],
    ) -> Result<Vec<DocumentManifest>> {
        let mut desired = Vec::new();
        for profile in profiles {
            if let Some(manifest) =
                self.rebuild_profile_manifest(storage, entity, source_version, profile)?
            {
                desired.push(manifest);
            }
        }
        let desired_keys = desired
            .iter()
            .map(|manifest| (manifest.profile_name.clone(), manifest.profile_version))
            .collect::<BTreeSet<_>>();
        for previous in store.list_for_entity(entity)? {
            if !desired_keys.contains(&(previous.profile_name.clone(), previous.profile_version)) {
                desired.push(DocumentManifest {
                    entity: entity.clone(),
                    profile_name: previous.profile_name,
                    profile_version: previous.profile_version,
                    source_version,
                    documents: Vec::new(),
                });
            }
        }
        Ok(desired)
    }

    fn rebuild_profile_manifest(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
        profile: &IndexProfileArtifact,
    ) -> Result<Option<DocumentManifest>> {
        let Some(fields) = matching_fields(storage, entity, profile)? else {
            return Ok(None);
        };
        let mut documents = profile_documents(entity, source_version, profile, &fields)?;
        documents.retain(|document| {
            profile
                .fields
                .get(&document.id.field)
                .is_some_and(|options| options.bm25.is_some())
        });
        Ok(Some(DocumentManifest {
            entity: entity.clone(),
            profile_name: profile.name.clone(),
            profile_version: profile.version,
            source_version,
            documents,
        }))
    }

    fn apply_changes(&self, changes: DocumentChanges) -> Result<()> {
        self.index.apply_document_changes(&changes)
    }
}

fn merge_changes(changes: Vec<DocumentChanges>) -> DocumentChanges {
    let mut upserts = BTreeMap::new();
    let mut deletes = BTreeSet::<IndexDocumentId>::new();
    for change in changes {
        for id in change.deletes {
            upserts.remove(&id);
            deletes.insert(id);
        }
        for document in change.upserts {
            deletes.remove(&document.id);
            upserts.insert(document.id.clone(), document);
        }
    }
    DocumentChanges {
        upserts: upserts.into_values().collect(),
        deletes: deletes.into_iter().collect(),
    }
}

impl IndexTarget for Bm25IndexTarget<'_> {
    fn apply(
        &self,
        storage: &Storage,
        event: &MutationEvent,
    ) -> std::result::Result<(), TargetApplyError> {
        let result = match &event.entity {
            Some(entity) => self.apply_entity(
                storage,
                entity,
                event.source_version,
                matches!(event.kind, MutationKind::Delete),
            ),
            None => self.rebuild(storage),
        };
        result.map_err(|error| TargetApplyError {
            message: error.to_string(),
            retryable: true,
        })
    }
}
