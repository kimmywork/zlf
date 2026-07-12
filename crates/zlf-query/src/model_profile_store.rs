use zlf_core::{Result, ZlfError};
use zlf_index::EmbeddingModelProfile;
use zlf_storage::{MutationSequence, Storage};

const NAMESPACE: &str = "embedding-model-profile";
const PREFIX: &str = "projection:embedding-model-profile:v1:";

pub struct EmbeddingModelProfileStore<'a> {
    storage: &'a Storage,
}

impl<'a> EmbeddingModelProfileStore<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn put(&self, profile: &EmbeddingModelProfile) -> Result<MutationSequence> {
        profile.validate_dense_v1().map_err(ZlfError::Internal)?;
        let key = profile_key(&profile.id, profile.version);
        if let Some(existing) = self.storage.get_raw(&key)? {
            let existing: EmbeddingModelProfile =
                bincode::deserialize(&existing).map_err(serialization)?;
            if existing == *profile {
                return self.storage.latest_mutation_sequence();
            }
            return Err(ZlfError::Internal(
                "embedding model profile identity is immutable".into(),
            ));
        }
        let record = (
            key.into_bytes(),
            bincode::serialize(profile).map_err(serialization)?,
        );
        self.storage.commit_projection_config(
            NAMESPACE,
            &format!("{}:{}", profile.id, profile.version),
            &[record],
        )
    }

    pub fn get(&self, id: &str, version: u32) -> Result<Option<EmbeddingModelProfile>> {
        self.storage
            .get_raw(&profile_key(id, version))?
            .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
            .transpose()
    }

    pub fn list(&self) -> Result<Vec<EmbeddingModelProfile>> {
        let mut profiles = self
            .storage
            .scan_prefix(PREFIX)?
            .into_iter()
            .map(|(_, bytes)| bincode::deserialize(&bytes).map_err(serialization))
            .collect::<Result<Vec<_>>>()?;
        profiles.sort_by(|left: &EmbeddingModelProfile, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.version.cmp(&right.version))
        });
        Ok(profiles)
    }
}

fn profile_key(id: &str, version: u32) -> String {
    format!("{PREFIX}{}:{version:010}", hex(id.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
