use std::collections::HashSet;

use chrono::{DateTime, Utc};
use zlf_core::{Result, ZlfError};
use zlf_index::{
    GenerationId, GenerationMetadata, GenerationState, IndexStatus, GENERATION_SCHEMA_VERSION,
};
use zlf_storage::Storage;

use crate::{CoordinatorConfig, IndexCoordinator};

const NAMESPACE: &str = "index-generation";

pub struct GenerationManager<'a> {
    storage: &'a Storage,
}

impl<'a> GenerationManager<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn create(&self, metadata: &GenerationMetadata) -> Result<()> {
        validate_new(metadata)?;
        let key = generation_key(&metadata.target, &metadata.id);
        if self.storage.get_raw(&key)?.is_some() {
            return Err(ZlfError::Internal("generation already exists".into()));
        }
        self.save(metadata)
    }

    pub fn start_build(&self, target: &str, id: &GenerationId) -> Result<GenerationMetadata> {
        self.transition(
            target,
            id,
            GenerationState::Draft,
            GenerationState::Building,
        )
    }

    pub fn checkpoint(
        &self,
        target: &str,
        id: &GenerationId,
        checkpoint: u64,
    ) -> Result<GenerationMetadata> {
        let mut metadata = self.required(target, id)?;
        if metadata.state != GenerationState::Building || checkpoint < metadata.build_checkpoint {
            return Err(ZlfError::Internal("invalid generation checkpoint".into()));
        }
        metadata.build_checkpoint = checkpoint;
        self.save(&metadata)?;
        Ok(metadata)
    }

    pub fn begin_validation(&self, target: &str, id: &GenerationId) -> Result<GenerationMetadata> {
        self.transition(
            target,
            id,
            GenerationState::Building,
            GenerationState::Validating,
        )
    }

    pub fn validation_passed(
        &self,
        target: &str,
        id: &GenerationId,
        document_count: u64,
        checksum: &str,
    ) -> Result<GenerationMetadata> {
        let mut metadata = self.required(target, id)?;
        if metadata.state != GenerationState::Validating || checksum.is_empty() {
            return Err(ZlfError::Internal("invalid generation validation".into()));
        }
        metadata.document_count = document_count;
        metadata.checksum = Some(checksum.to_string());
        metadata.validated_at = Some(Utc::now());
        self.save(&metadata)?;
        Ok(metadata)
    }

    pub fn fail(
        &self,
        target: &str,
        id: &GenerationId,
        message: &str,
    ) -> Result<GenerationMetadata> {
        let mut metadata = self.required(target, id)?;
        if matches!(
            metadata.state,
            GenerationState::Active | GenerationState::Retired
        ) {
            return Err(ZlfError::Internal(
                "published generation cannot fail".into(),
            ));
        }
        metadata.state = GenerationState::Failed;
        metadata.failure = Some(message.chars().take(256).collect());
        self.save(&metadata)?;
        Ok(metadata)
    }

    pub fn activate(&self, target: &str, id: &GenerationId) -> Result<u64> {
        let mut next = self.required(target, id)?;
        if next.state != GenerationState::Validating
            || next.checksum.is_none()
            || next.validated_at.is_none()
        {
            return Err(ZlfError::Internal(
                "only a validated generation can activate".into(),
            ));
        }
        let mut records = Vec::new();
        if let Some(mut previous) = self.active(target)? {
            previous.state = GenerationState::Retired;
            records.push(serialized_record(
                generation_key(target, &previous.id),
                &previous,
            )?);
        }
        next.state = GenerationState::Active;
        records.push(serialized_record(generation_key(target, &next.id), &next)?);
        records.push((
            active_key(target).into_bytes(),
            next.id.0.as_bytes().to_vec(),
        ));
        self.storage
            .commit_projection_config(NAMESPACE, &next.id.0, &records)
    }

    pub fn get(&self, target: &str, id: &GenerationId) -> Result<Option<GenerationMetadata>> {
        self.storage
            .get_raw(&generation_key(target, id))?
            .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
            .transpose()
    }

    pub fn active(&self, target: &str) -> Result<Option<GenerationMetadata>> {
        let Some(id) = self.storage.get_raw(&active_key(target))? else {
            return Ok(None);
        };
        let id =
            String::from_utf8(id).map_err(|error| ZlfError::Serialization(error.to_string()))?;
        self.get(target, &GenerationId(id))
    }

    pub fn list(&self, target: &str) -> Result<Vec<GenerationMetadata>> {
        let mut generations = self
            .storage
            .scan_prefix(&generation_prefix(target))?
            .into_iter()
            .map(|(_, bytes)| {
                bincode::deserialize::<GenerationMetadata>(&bytes).map_err(serialization)
            })
            .collect::<Result<Vec<_>>>()?;
        generations.sort_by_key(|metadata| metadata.created_at);
        Ok(generations)
    }

    pub fn prune(&self, target: &str, now: DateTime<Utc>) -> Result<usize> {
        let ids = prunable_generation_ids(&self.list(target)?, now);
        for id in &ids {
            self.storage.delete_raw(&generation_key(target, id))?;
        }
        Ok(ids.len())
    }

    pub fn status(&self, target: &str) -> Result<IndexStatus> {
        let progress =
            IndexCoordinator::new(self.storage, CoordinatorConfig::default()).progress(target)?;
        let active = self.active(target)?;
        Ok(IndexStatus {
            target: target.to_string(),
            active_generation: active.as_ref().map(|metadata| metadata.id.clone()),
            scanned_watermark: progress.scanned_watermark,
            published_watermark: progress.published_watermark,
            state: active.as_ref().map(|metadata| metadata.state),
            document_count: active.map_or(0, |metadata| metadata.document_count),
        })
    }

    fn required(&self, target: &str, id: &GenerationId) -> Result<GenerationMetadata> {
        self.get(target, id)?
            .ok_or_else(|| ZlfError::Internal("generation not found".into()))
    }

    fn transition(
        &self,
        target: &str,
        id: &GenerationId,
        from: GenerationState,
        to: GenerationState,
    ) -> Result<GenerationMetadata> {
        let mut metadata = self.required(target, id)?;
        if metadata.state != from {
            return Err(ZlfError::Internal("invalid generation transition".into()));
        }
        metadata.state = to;
        self.save(&metadata)?;
        Ok(metadata)
    }

    fn save(&self, metadata: &GenerationMetadata) -> Result<()> {
        self.storage.put_raw(
            &generation_key(&metadata.target, &metadata.id),
            &bincode::serialize(metadata).map_err(serialization)?,
        )
    }
}

fn prunable_generation_ids(
    generations: &[GenerationMetadata],
    now: DateTime<Utc>,
) -> Vec<GenerationId> {
    let active = generations
        .iter()
        .find(|item| item.state == GenerationState::Active)
        .map(|item| item.id.clone());
    let previous = generations
        .iter()
        .rev()
        .find(|item| item.state == GenerationState::Retired)
        .map(|item| item.id.clone());
    let protected = [active, previous]
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();
    let failed = generations
        .iter()
        .filter(|item| item.state == GenerationState::Failed)
        .collect::<Vec<_>>();
    generations
        .iter()
        .filter(|item| {
            let old_retired = item.state == GenerationState::Retired;
            let failed_index = failed.iter().position(|failed| failed.id == item.id);
            let old_failed = failed_index.is_some_and(|index| {
                now.signed_duration_since(item.created_at) > chrono::Duration::days(30)
                    || failed.len().saturating_sub(index) > 100
            });
            (old_retired || old_failed) && !protected.contains(&item.id)
        })
        .map(|item| item.id.clone())
        .collect()
}

fn validate_new(metadata: &GenerationMetadata) -> Result<()> {
    if metadata.schema_version != GENERATION_SCHEMA_VERSION
        || metadata.id.0.is_empty()
        || metadata.target.is_empty()
        || metadata.profile_name.is_empty()
        || metadata.state != GenerationState::Draft
    {
        return Err(ZlfError::Internal("invalid draft generation".into()));
    }
    Ok(())
}

fn generation_prefix(target: &str) -> String {
    format!(
        "projection:{NAMESPACE}:generation:{}:",
        hex(target.as_bytes())
    )
}

fn generation_key(target: &str, id: &GenerationId) -> String {
    format!("{}{}", generation_prefix(target), hex(id.0.as_bytes()))
}

fn active_key(target: &str) -> String {
    format!("projection:{NAMESPACE}:active:{}", hex(target.as_bytes()))
}

fn serialized_record(key: String, metadata: &GenerationMetadata) -> Result<(Vec<u8>, Vec<u8>)> {
    Ok((
        key.into_bytes(),
        bincode::serialize(metadata).map_err(serialization)?,
    ))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
