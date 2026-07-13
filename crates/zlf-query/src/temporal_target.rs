use serde::{Deserialize, Serialize};
use zlf_core::{EntityRef, Result, Value};
use zlf_index::{
    EventRecord, EventTimeStore, GenerationId, IndexProfileArtifact, ValidityRecord, ValidityStore,
};
use zlf_storage::{MutationEvent, MutationKind, Storage};

use crate::coordinator::{IndexTarget, TargetApplyError};
use crate::fake_documents::{active_profiles, matching_fields};
use crate::temporal_manifest_store::{list_manifests, load_manifest, manifest_key, save_manifest};
use crate::temporal_projection::project_manifest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TemporalManifest {
    pub(crate) entity: EntityRef,
    pub(crate) profile_name: String,
    pub(crate) profile_version: u32,
    pub(crate) source_version: u64,
    pub(crate) events: Vec<EventRecord>,
    pub(crate) validities: Vec<ValidityRecord>,
}

pub struct TemporalIndexTarget<'a> {
    events: &'a EventTimeStore,
    validities: &'a ValidityStore,
    generation: GenerationId,
}

impl<'a> TemporalIndexTarget<'a> {
    pub fn new(
        events: &'a EventTimeStore,
        validities: &'a ValidityStore,
        generation: GenerationId,
    ) -> Self {
        Self {
            events,
            validities,
            generation,
        }
    }

    pub fn rebuild(&self, storage: &Storage) -> Result<()> {
        for node in storage.get_all_nodes()? {
            let entity = EntityRef::Node(node.id);
            self.apply_entity(storage, &entity, source_version(storage, &entity)?, false)?;
        }
        for edge in storage.get_all_edges()? {
            let entity = EntityRef::Edge(edge.id);
            self.apply_entity(storage, &entity, source_version(storage, &entity)?, false)?;
        }
        Ok(())
    }

    fn apply_entity(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
        delete: bool,
    ) -> Result<()> {
        if delete {
            return self.delete_entity(storage, entity);
        }
        for profile in active_profiles(storage)? {
            let Some(fields) = matching_fields(storage, entity, &profile)? else {
                continue;
            };
            self.apply_profile(storage, entity, source_version, &profile, &fields)?;
        }
        Ok(())
    }

    fn apply_profile(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
        profile: &IndexProfileArtifact,
        fields: &std::collections::HashMap<String, Value>,
    ) -> Result<()> {
        self.retire_versions(storage, entity, &profile.name, profile.version)?;
        let desired = project_manifest(&self.generation, entity, source_version, profile, fields)?;
        if let Some(previous) = load_manifest(storage, entity, &profile.name, profile.version)? {
            self.remove_records(&previous)?;
        }
        self.put_records(&desired)?;
        save_manifest(storage, &desired)
    }

    fn retire_versions(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        name: &str,
        version: u32,
    ) -> Result<()> {
        for manifest in list_manifests(storage, entity)?
            .into_iter()
            .filter(|item| item.profile_name == name && item.profile_version != version)
        {
            self.remove_records(&manifest)?;
            storage.delete_raw(&manifest_key(
                entity,
                &manifest.profile_name,
                manifest.profile_version,
            ))?;
        }
        Ok(())
    }

    fn delete_entity(&self, storage: &Storage, entity: &EntityRef) -> Result<()> {
        for manifest in list_manifests(storage, entity)? {
            self.remove_records(&manifest)?;
            storage.delete_raw(&manifest_key(
                entity,
                &manifest.profile_name,
                manifest.profile_version,
            ))?;
        }
        Ok(())
    }

    fn put_records(&self, manifest: &TemporalManifest) -> Result<()> {
        for record in &manifest.events {
            self.events.put(record)?;
        }
        for record in &manifest.validities {
            self.validities.put(record)?;
        }
        Ok(())
    }

    fn remove_records(&self, manifest: &TemporalManifest) -> Result<()> {
        for record in &manifest.events {
            self.events.delete(record)?;
        }
        for record in &manifest.validities {
            self.validities.delete(record)?;
        }
        Ok(())
    }
}

impl IndexTarget for TemporalIndexTarget<'_> {
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

fn source_version(storage: &Storage, entity: &EntityRef) -> Result<u64> {
    Ok(storage
        .get_entity_state(entity)?
        .map_or(0, |state| state.source_version))
}
