use zlf_core::{EntityRef, Result, ZlfError};
use zlf_storage::Storage;

use crate::temporal_target::TemporalManifest;

const PREFIX: &str = "projection:temporal-manifest:v1:";

pub(crate) fn save_manifest(storage: &Storage, manifest: &TemporalManifest) -> Result<()> {
    storage.put_raw(
        &manifest_key(
            &manifest.entity,
            &manifest.profile_name,
            manifest.profile_version,
        ),
        &bincode::serialize(manifest).map_err(serialization)?,
    )
}

pub(crate) fn load_manifest(
    storage: &Storage,
    entity: &EntityRef,
    name: &str,
    version: u32,
) -> Result<Option<TemporalManifest>> {
    storage
        .get_raw(&manifest_key(entity, name, version))?
        .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
        .transpose()
}

pub(crate) fn list_manifests(
    storage: &Storage,
    entity: &EntityRef,
) -> Result<Vec<TemporalManifest>> {
    storage
        .scan_prefix(&entity_prefix(entity))?
        .into_iter()
        .map(|(_, bytes)| bincode::deserialize(&bytes).map_err(serialization))
        .collect()
}

pub(crate) fn manifest_key(entity: &EntityRef, name: &str, version: u32) -> String {
    format!(
        "{}{}:{version:010}",
        entity_prefix(entity),
        hex(name.as_bytes())
    )
}

fn entity_prefix(entity: &EntityRef) -> String {
    let kind = match entity {
        EntityRef::Node(_) => "node",
        EntityRef::Edge(_) => "edge",
    };
    format!("{PREFIX}{kind}:{}:", hex(entity.id().as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
