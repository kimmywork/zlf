use rocksdb::{Direction, IteratorMode, WriteBatch};
use zlf_core::{EntityRef, Result, ZlfError};

use crate::{EntityState, MutationEvent, MutationSequence, Storage};

const SCHEMA_KEY: &[u8] = b"meta:index-lifecycle:schema";
pub(crate) const NEXT_SEQUENCE_KEY: &[u8] = b"meta:index-lifecycle:next-sequence";
const OUTBOX_PREFIX: &[u8] = b"outbox:";
const ENTITY_STATE_PREFIX: &[u8] = b"entity-state:";
const LIFECYCLE_SCHEMA_VERSION: u32 = 1;

impl Storage {
    pub(crate) fn initialize_lifecycle(&self, existing: bool) -> Result<()> {
        match self.db.get(SCHEMA_KEY).map_err(internal)? {
            Some(value) if decode_u32(&value)? == LIFECYCLE_SCHEMA_VERSION => {}
            Some(value) => {
                return Err(ZlfError::Internal(format!(
                    "unsupported storage lifecycle schema: {}",
                    decode_u32(&value)?
                )));
            }
            None if existing => {
                return Err(ZlfError::Internal(
                    "database does not contain the first-version lifecycle schema".into(),
                ));
            }
            None => {
                let mut batch = WriteBatch::default();
                batch.put(SCHEMA_KEY, LIFECYCLE_SCHEMA_VERSION.to_be_bytes());
                batch.put(NEXT_SEQUENCE_KEY, 0_u64.to_be_bytes());
                self.db.write(batch).map_err(internal)?;
            }
        }
        Ok(())
    }

    pub fn latest_mutation_sequence(&self) -> Result<MutationSequence> {
        self.db
            .get(NEXT_SEQUENCE_KEY)
            .map_err(internal)?
            .map_or(Ok(0), |bytes| decode_u64(&bytes))
    }

    pub fn get_entity_state(&self, entity: &EntityRef) -> Result<Option<EntityState>> {
        self.db
            .get(entity_state_key(entity))
            .map_err(internal)?
            .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
            .transpose()
    }

    pub fn mutation_events_after(
        &self,
        sequence: MutationSequence,
        limit: usize,
    ) -> Result<Vec<MutationEvent>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let start = outbox_key(sequence.saturating_add(1));
        let iter = self
            .db
            .iterator(IteratorMode::From(&start, Direction::Forward));
        let mut events = Vec::new();
        for item in iter {
            let (key, value) = item.map_err(internal)?;
            if !key.starts_with(OUTBOX_PREFIX) || events.len() == limit {
                break;
            }
            events.push(bincode::deserialize(&value).map_err(serialization)?);
        }
        Ok(events)
    }

    pub fn compact_outbox_through(&self, sequence: MutationSequence) -> Result<usize> {
        if sequence == 0 {
            return Ok(0);
        }
        let _guard = self.write_guard()?;
        let mut batch = WriteBatch::default();
        let mut count = 0;
        for item in self.db.iterator(IteratorMode::Start) {
            let (key, _) = item.map_err(internal)?;
            if !key.starts_with(OUTBOX_PREFIX) {
                continue;
            }
            let event_sequence = decode_u64(&key[OUTBOX_PREFIX.len()..])?;
            if event_sequence > sequence {
                break;
            }
            batch.delete(key);
            count += 1;
        }
        if count > 0 {
            self.db.write(batch).map_err(internal)?;
        }
        Ok(count)
    }
}

pub(crate) fn outbox_key(sequence: MutationSequence) -> Vec<u8> {
    let mut key = OUTBOX_PREFIX.to_vec();
    key.extend_from_slice(&sequence.to_be_bytes());
    key
}

pub(crate) fn entity_state_key(entity: &EntityRef) -> Vec<u8> {
    let mut key = ENTITY_STATE_PREFIX.to_vec();
    key.push(match entity {
        EntityRef::Node(_) => 0,
        EntityRef::Edge(_) => 1,
    });
    key.extend_from_slice(&(entity.id().len() as u32).to_be_bytes());
    key.extend_from_slice(entity.id().as_bytes());
    key
}

fn decode_u32(bytes: &[u8]) -> Result<u32> {
    bytes
        .try_into()
        .map(u32::from_be_bytes)
        .map_err(|_| ZlfError::Serialization("invalid u32 metadata".into()))
}

fn decode_u64(bytes: &[u8]) -> Result<u64> {
    bytes
        .try_into()
        .map(u64::from_be_bytes)
        .map_err(|_| ZlfError::Serialization("invalid u64 metadata".into()))
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
