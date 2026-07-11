use zlf_core::{EntityRef, Result, ZlfError};
use zlf_index::{reconcile_manifest, DocumentChanges, DocumentManifest};
use zlf_storage::Storage;

pub struct IndexManifestStore<'a> {
    storage: &'a Storage,
}

impl<'a> IndexManifestStore<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn get(
        &self,
        entity: &EntityRef,
        profile_name: &str,
        profile_version: u32,
    ) -> Result<Option<DocumentManifest>> {
        self.storage
            .get_raw(&manifest_key(entity, profile_name, profile_version))?
            .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
            .transpose()
    }

    pub fn reconcile_and_save(&self, desired: &DocumentManifest) -> Result<DocumentChanges> {
        let previous = self.get(
            &desired.entity,
            &desired.profile_name,
            desired.profile_version,
        )?;
        let changes = reconcile_manifest(previous.as_ref(), desired).map_err(ZlfError::Internal)?;
        if changes != DocumentChanges::default() || previous.is_none() {
            self.storage.put_raw(
                &manifest_key(
                    &desired.entity,
                    &desired.profile_name,
                    desired.profile_version,
                ),
                &bincode::serialize(desired).map_err(serialization)?,
            )?;
        }
        Ok(changes)
    }

    pub fn list_for_entity(&self, entity: &EntityRef) -> Result<Vec<DocumentManifest>> {
        let prefix = entity_manifest_prefix(entity);
        self.storage
            .scan_prefix(&prefix)?
            .into_iter()
            .map(|(_, bytes)| bincode::deserialize(&bytes).map_err(serialization))
            .collect()
    }

    pub fn delete(
        &self,
        entity: &EntityRef,
        profile_name: &str,
        profile_version: u32,
    ) -> Result<()> {
        self.storage
            .delete_raw(&manifest_key(entity, profile_name, profile_version))
    }
}

fn manifest_key(entity: &EntityRef, profile_name: &str, profile_version: u32) -> String {
    format!(
        "{}{}:{profile_version:010}",
        entity_manifest_prefix(entity),
        hex(profile_name.as_bytes())
    )
}

fn entity_manifest_prefix(entity: &EntityRef) -> String {
    let (kind, id) = match entity {
        EntityRef::Node(id) => ("node", id),
        EntityRef::Edge(id) => ("edge", id),
    };
    format!("projection:index-manifest:{kind}:{}:", hex(id.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
