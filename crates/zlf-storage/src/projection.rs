use rocksdb::WriteBatch;
use zlf_core::{Result, ZlfError};

use crate::outbox::{outbox_key, NEXT_SEQUENCE_KEY};
use crate::{
    MutationEvent, MutationKind, MutationSequence, Storage, MUTATION_EVENT_SCHEMA_VERSION,
};

impl Storage {
    pub fn commit_projection_config(
        &self,
        namespace: &str,
        artifact_ref: &str,
        records: &[(Vec<u8>, Vec<u8>)],
    ) -> Result<MutationSequence> {
        validate_projection_records(namespace, artifact_ref, records)?;
        let _guard = self.write_guard()?;
        let sequence = self
            .latest_mutation_sequence()?
            .checked_add(1)
            .ok_or_else(|| ZlfError::Internal("mutation sequence exhausted".into()))?;
        let event = MutationEvent {
            schema_version: MUTATION_EVENT_SCHEMA_VERSION,
            sequence,
            entity: None,
            source_version: sequence,
            kind: MutationKind::ConfigurationChanged {
                namespace: namespace.to_string(),
                artifact_ref: artifact_ref.to_string(),
            },
            occurred_at: chrono::Utc::now(),
        };
        let mut batch = WriteBatch::default();
        for (key, value) in records {
            batch.put(key, value);
        }
        batch.put(NEXT_SEQUENCE_KEY, sequence.to_be_bytes());
        batch.put(
            outbox_key(sequence),
            bincode::serialize(&event).map_err(serialization)?,
        );
        self.db.write(batch).map_err(internal)?;
        Ok(sequence)
    }
}

fn validate_projection_records(
    namespace: &str,
    artifact_ref: &str,
    records: &[(Vec<u8>, Vec<u8>)],
) -> Result<()> {
    if namespace.is_empty()
        || artifact_ref.is_empty()
        || records.is_empty()
        || !namespace
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ZlfError::Internal(
            "invalid projection configuration".into(),
        ));
    }
    let prefix = format!("projection:{namespace}:");
    if records
        .iter()
        .any(|(key, _)| !key.starts_with(prefix.as_bytes()))
    {
        return Err(ZlfError::Internal(
            "projection record escaped its namespace".into(),
        ));
    }
    Ok(())
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
