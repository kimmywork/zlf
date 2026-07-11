use zlf_core::{Result, ZlfError};
use zlf_index::IndexProfileArtifact;
use zlf_storage::{MutationSequence, Storage};

use crate::ZlfDatabase;

const NAMESPACE: &str = "index-profile";

pub struct IndexProfileStore<'a> {
    storage: &'a Storage,
}

impl<'a> IndexProfileStore<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn put(&self, profile: &IndexProfileArtifact) -> Result<MutationSequence> {
        profile.validate().map_err(ZlfError::Internal)?;
        let key = artifact_key(&profile.name, profile.version);
        let value = bincode::serialize(profile).map_err(serialization)?;
        if let Some(existing) = self.storage.get_raw(&key)? {
            if existing == value {
                return self.sequence_for_existing(&profile.name, profile.version);
            }
            return Err(ZlfError::Internal(format!(
                "index profile {} version {} is immutable",
                profile.name, profile.version
            )));
        }
        self.storage.commit_projection_config(
            NAMESPACE,
            &artifact_ref(&profile.name, profile.version),
            &[(key.into_bytes(), value)],
        )
    }

    pub fn get(&self, name: &str, version: u32) -> Result<Option<IndexProfileArtifact>> {
        self.storage
            .get_raw(&artifact_key(name, version))?
            .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
            .transpose()
    }

    pub fn activate(&self, name: &str, version: u32) -> Result<MutationSequence> {
        if self.get(name, version)?.is_none() {
            return Err(ZlfError::Internal(format!(
                "index profile not found: {name}/{version}"
            )));
        }
        let reference = artifact_ref(name, version);
        self.storage.commit_projection_config(
            NAMESPACE,
            &reference,
            &[(active_key(name).into_bytes(), reference.as_bytes().to_vec())],
        )
    }

    pub fn active(&self, name: &str) -> Result<Option<IndexProfileArtifact>> {
        let Some(reference) = self.storage.get_raw(&active_key(name))? else {
            return Ok(None);
        };
        let reference = String::from_utf8(reference)
            .map_err(|error| ZlfError::Serialization(error.to_string()))?;
        let (stored_name, version) = parse_artifact_ref(&reference)?;
        self.get(&stored_name, version)
    }

    pub fn list(&self) -> Result<Vec<IndexProfileArtifact>> {
        let prefix = format!("projection:{NAMESPACE}:artifact:");
        let mut profiles = self
            .storage
            .scan_prefix(&prefix)?
            .into_iter()
            .map(|(_, bytes)| {
                bincode::deserialize::<IndexProfileArtifact>(&bytes).map_err(serialization)
            })
            .collect::<Result<Vec<_>>>()?;
        profiles
            .sort_by(|left, right| (&left.name, left.version).cmp(&(&right.name, right.version)));
        Ok(profiles)
    }

    fn sequence_for_existing(&self, name: &str, version: u32) -> Result<MutationSequence> {
        let reference = artifact_ref(name, version);
        Ok(self
            .storage
            .mutation_events_after(0, usize::MAX)?
            .into_iter()
            .find_map(|event| match event.kind {
                zlf_storage::MutationKind::ConfigurationChanged {
                    namespace,
                    artifact_ref,
                } if namespace == NAMESPACE && artifact_ref == reference => Some(event.sequence),
                _ => None,
            })
            .unwrap_or_default())
    }
}

impl ZlfDatabase {
    pub fn put_index_profile(&self, profile: &IndexProfileArtifact) -> Result<MutationSequence> {
        IndexProfileStore::new(&self.storage).put(profile)
    }

    pub fn activate_index_profile(&self, name: &str, version: u32) -> Result<MutationSequence> {
        IndexProfileStore::new(&self.storage).activate(name, version)
    }

    pub fn active_index_profile(&self, name: &str) -> Result<Option<IndexProfileArtifact>> {
        IndexProfileStore::new(&self.storage).active(name)
    }

    pub fn index_profiles(&self) -> Result<Vec<IndexProfileArtifact>> {
        IndexProfileStore::new(&self.storage).list()
    }
}

fn artifact_key(name: &str, version: u32) -> String {
    format!(
        "projection:{NAMESPACE}:artifact:{}:{version:010}",
        hex(name.as_bytes())
    )
}

fn active_key(name: &str) -> String {
    format!("projection:{NAMESPACE}:active:{}", hex(name.as_bytes()))
}

fn artifact_ref(name: &str, version: u32) -> String {
    format!("{}:{version}", hex(name.as_bytes()))
}

fn parse_artifact_ref(reference: &str) -> Result<(String, u32)> {
    let (name, version) = reference
        .rsplit_once(':')
        .ok_or_else(|| ZlfError::Serialization("invalid profile reference".into()))?;
    let bytes = (0..name.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&name[index..index + 2], 16))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| ZlfError::Serialization(error.to_string()))?;
    let name =
        String::from_utf8(bytes).map_err(|error| ZlfError::Serialization(error.to_string()))?;
    let version = version
        .parse()
        .map_err(|error: std::num::ParseIntError| ZlfError::Serialization(error.to_string()))?;
    Ok((name, version))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
