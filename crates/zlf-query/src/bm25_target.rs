use zlf_core::{EntityRef, Result};
use zlf_index::{BM25Index, DocumentChanges, DocumentManifest};
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

    fn rebuild_current(&self, storage: &Storage) -> Result<()> {
        for node in storage.get_all_nodes()? {
            let entity = EntityRef::Node(node.id);
            let version = storage
                .get_entity_state(&entity)?
                .map_or(0, |state| state.source_version);
            self.apply_entity(storage, &entity, version, false)?;
        }
        for edge in storage.get_all_edges()? {
            let entity = EntityRef::Edge(edge.id);
            let version = storage
                .get_entity_state(&entity)?
                .map_or(0, |state| state.source_version);
            self.apply_entity(storage, &entity, version, false)?;
        }
        Ok(())
    }

    fn apply_changes(&self, changes: DocumentChanges) -> Result<()> {
        for id in changes.deletes {
            self.index.remove_document(&id)?;
        }
        for document in changes.upserts {
            self.index.index_document(&document)?;
        }
        Ok(())
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
            None => self.rebuild_current(storage),
        };
        result.map_err(|error| TargetApplyError {
            message: error.to_string(),
            retryable: true,
        })
    }
}
