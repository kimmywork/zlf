use zlf_core::{Result, ZlfError};
use zlf_index::{IndexProfileArtifact, INDEX_PROFILE_SCHEMA_VERSION};
use zlf_prolog::Term;
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
        let mut profile = profile.clone();
        if profile.source_hash.is_empty() {
            profile.refresh_source_hash();
        }
        profile.validate().map_err(ZlfError::Internal)?;
        let key = artifact_key(&profile.name, profile.version);
        let value = bincode::serialize(&profile).map_err(serialization)?;
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
        let sequence = IndexProfileStore::new(&self.storage).put(profile)?;
        self.catch_up_bm25()?;
        Ok(sequence)
    }

    pub fn activate_index_profile(&self, name: &str, version: u32) -> Result<MutationSequence> {
        let sequence = IndexProfileStore::new(&self.storage).activate(name, version)?;
        self.catch_up_bm25()?;
        Ok(sequence)
    }

    pub fn active_index_profile(&self, name: &str) -> Result<Option<IndexProfileArtifact>> {
        IndexProfileStore::new(&self.storage).active(name)
    }

    pub fn index_profiles(&self) -> Result<Vec<IndexProfileArtifact>> {
        IndexProfileStore::new(&self.storage).list()
    }
}

pub(crate) fn lower_profile_directive(
    name: &Term,
    version: &Term,
    config: &Term,
) -> Result<IndexProfileArtifact> {
    let name = term_text(name)?;
    let version = match version {
        Term::Integer(value) => u32::try_from(*value)
            .map_err(|_| ZlfError::Internal("profile version must be a positive u32".into()))?,
        _ => {
            return Err(ZlfError::Internal(
                "profile version must be an integer".into(),
            ))
        }
    };
    let mut value = term_json(config)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| ZlfError::Internal("profile config must be an object".into()))?;
    object.insert("schema_version".into(), INDEX_PROFILE_SCHEMA_VERSION.into());
    object.insert("name".into(), name.into());
    object.insert("version".into(), version.into());
    object.insert("created_at".into(), chrono::Utc::now().to_rfc3339().into());
    object
        .entry("source_hash")
        .or_insert_with(|| serde_json::Value::String(String::new()));
    let mut profile: IndexProfileArtifact =
        serde_json::from_value(value).map_err(|error| ZlfError::InvalidJson(error.to_string()))?;
    if profile.source_hash.is_empty() {
        profile.refresh_source_hash();
    }
    Ok(profile)
}

fn term_json(term: &Term) -> Result<serde_json::Value> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value.clone().into()),
        Term::Integer(value) => Ok((*value).into()),
        Term::Float(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .ok_or_else(|| ZlfError::InvalidJson("non-finite profile number".into())),
        Term::List(values) => values
            .iter()
            .map(term_json)
            .collect::<Result<Vec<_>>>()
            .map(serde_json::Value::Array),
        Term::Object(values) => values
            .iter()
            .map(|(key, value)| Ok((key.clone(), term_json(value)?)))
            .collect::<Result<serde_json::Map<_, _>>>()
            .map(serde_json::Value::Object),
        _ => Err(ZlfError::InvalidJson(
            "unsupported term in profile config".into(),
        )),
    }
}

fn term_text(term: &Term) -> Result<String> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value.clone()),
        _ => Err(ZlfError::Internal("profile name must be text".into())),
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
