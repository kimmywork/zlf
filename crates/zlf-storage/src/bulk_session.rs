use rocksdb::WriteBatch;
use serde::{Deserialize, Serialize};
use zlf_core::{Result, ZlfError};

use crate::outbox::{outbox_key, NEXT_SEQUENCE_KEY};
use crate::{
    MutationEvent, MutationKind, MutationSequence, Storage, StorageRecordPlan,
    MUTATION_EVENT_SCHEMA_VERSION,
};

const BULK_SESSION_PREFIX: &str = "bulk-session:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BulkSessionState {
    Started,
    Writing,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkSession {
    pub id: String,
    pub state: BulkSessionState,
    pub checkpoint: u64,
    pub rebuild_sequence: Option<MutationSequence>,
}

impl Storage {
    pub fn begin_bulk_session(&self, id: &str) -> Result<BulkSession> {
        let _guard = self.write_guard()?;
        if let Some(session) = self.get_bulk_session(id)? {
            return Ok(session);
        }
        let session = BulkSession {
            id: id.to_string(),
            state: BulkSessionState::Started,
            checkpoint: 0,
            rebuild_sequence: None,
        };
        self.db
            .put(session_key(id), serialize(&session)?)
            .map_err(internal)?;
        Ok(session)
    }

    pub fn get_bulk_session(&self, id: &str) -> Result<Option<BulkSession>> {
        self.db
            .get(session_key(id))
            .map_err(internal)?
            .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
            .transpose()
    }

    pub fn list_bulk_sessions(&self) -> Result<Vec<BulkSession>> {
        self.scan_prefix(BULK_SESSION_PREFIX)?
            .into_iter()
            .map(|(_, bytes)| bincode::deserialize(&bytes).map_err(serialization))
            .collect()
    }

    pub fn write_bulk_plan(
        &self,
        id: &str,
        plan: &StorageRecordPlan,
        checkpoint: u64,
    ) -> Result<usize> {
        let _guard = self.write_guard()?;
        let mut session = self
            .get_bulk_session(id)?
            .ok_or_else(|| ZlfError::Internal(format!("bulk session not found: {id}")))?;
        if session.state == BulkSessionState::Complete {
            return Err(ZlfError::Internal(format!(
                "bulk session already complete: {id}"
            )));
        }
        if checkpoint < session.checkpoint {
            return Err(ZlfError::Internal("bulk checkpoint moved backwards".into()));
        }
        let mut batch = WriteBatch::default();
        for record in &plan.records {
            batch.put(&record.key, &record.value);
        }
        session.state = BulkSessionState::Writing;
        session.checkpoint = checkpoint;
        batch.put(session_key(id), serialize(&session)?);
        self.db.write(batch).map_err(internal)?;
        Ok(plan.records.len())
    }

    pub fn complete_bulk_session(&self, id: &str) -> Result<MutationSequence> {
        let _guard = self.write_guard()?;
        let mut session = self
            .get_bulk_session(id)?
            .ok_or_else(|| ZlfError::Internal(format!("bulk session not found: {id}")))?;
        if let Some(sequence) = session.rebuild_sequence {
            return Ok(sequence);
        }
        let sequence = self
            .latest_mutation_sequence()?
            .checked_add(1)
            .ok_or_else(|| ZlfError::Internal("mutation sequence exhausted".into()))?;
        session.state = BulkSessionState::Complete;
        session.rebuild_sequence = Some(sequence);
        let event = MutationEvent {
            schema_version: MUTATION_EVENT_SCHEMA_VERSION,
            sequence,
            entity: None,
            source_version: sequence,
            kind: MutationKind::RebuildRequired {
                bulk_id: id.to_string(),
            },
            occurred_at: chrono::Utc::now(),
        };
        let mut batch = WriteBatch::default();
        batch.put(session_key(id), serialize(&session)?);
        batch.put(NEXT_SEQUENCE_KEY, sequence.to_be_bytes());
        batch.put(outbox_key(sequence), serialize(&event)?);
        self.db.write(batch).map_err(internal)?;
        Ok(sequence)
    }
}

fn session_key(id: &str) -> String {
    format!("{BULK_SESSION_PREFIX}{id}")
}

fn serialize(value: &impl Serialize) -> Result<Vec<u8>> {
    bincode::serialize(value).map_err(serialization)
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
