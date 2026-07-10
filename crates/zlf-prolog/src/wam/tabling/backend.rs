use std::fmt::Debug;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use zlf_storage::{RawMutation, Storage};

use super::{TableAnswer, TableKey, TableState};
use crate::wam::error::{WamError, WamResult};

#[derive(Debug, Clone)]
pub struct PersistedTable {
    pub key: TableKey,
    pub state: TableState,
    pub answers: Vec<TableAnswer>,
    pub generation: u64,
}

pub trait TableBackend: Debug + Send + Sync {
    fn load(&self, key: &TableKey) -> WamResult<Option<PersistedTable>>;
    fn store_complete(&self, table: &PersistedTable) -> WamResult<()>;
    fn invalidate_all(&self) -> WamResult<()>;
}

pub struct RocksTableBackend {
    storage: Arc<Storage>,
}

impl std::fmt::Debug for RocksTableBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("RocksTableBackend").finish()
    }
}

impl RocksTableBackend {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    fn load_metadata(&self, key: &TableKey) -> WamResult<Option<PersistedMetadata>> {
        let Some(bytes) = self
            .storage
            .get_raw(&metadata_key(key))
            .map_err(backend_error)?
        else {
            return Ok(None);
        };
        let mut metadata: PersistedMetadata =
            bincode::deserialize(&bytes).map_err(backend_error)?;
        if metadata.key != *key {
            return Err(table_error("persistent table metadata key mismatch"));
        }
        if metadata.format_version != 2 {
            metadata.state = TableState::Stale;
        }
        Ok(Some(metadata))
    }

    fn load_answers(&self, key: &TableKey) -> WamResult<Vec<TableAnswer>> {
        self.storage
            .scan_prefix(&answer_prefix(key))
            .map_err(backend_error)?
            .into_iter()
            .map(|(_, bytes)| bincode::deserialize(&bytes).map_err(backend_error))
            .collect()
    }

    fn answer_mutations(&self, table: &PersistedTable) -> WamResult<Vec<RawMutation>> {
        let prefix = answer_prefix(&table.key);
        let mut mutations = self
            .storage
            .scan_prefix(&prefix)
            .map_err(backend_error)?
            .into_iter()
            .map(|(key, _)| RawMutation::Delete(key.into_bytes()))
            .collect::<Vec<_>>();
        for (index, answer) in table.answers.iter().enumerate() {
            mutations.push(answer_mutation(&prefix, index, answer)?);
        }
        Ok(mutations)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedMetadata {
    format_version: u32,
    key: TableKey,
    state: TableState,
    generation: u64,
    answer_count: u64,
}

impl TableBackend for RocksTableBackend {
    fn load(&self, key: &TableKey) -> WamResult<Option<PersistedTable>> {
        let Some(metadata) = self.load_metadata(key)? else {
            return Ok(None);
        };
        if metadata.state != TableState::Complete {
            return Ok(Some(PersistedTable {
                key: key.clone(),
                state: TableState::Stale,
                answers: Vec::new(),
                generation: metadata.generation,
            }));
        }
        let answers = self.load_answers(key)?;
        if answers.len() as u64 != metadata.answer_count {
            return Err(table_error("persistent table answer count mismatch"));
        }
        Ok(Some(PersistedTable {
            key: key.clone(),
            state: TableState::Complete,
            answers,
            generation: metadata.generation,
        }))
    }

    fn store_complete(&self, table: &PersistedTable) -> WamResult<()> {
        let mut mutations = self.answer_mutations(table)?;
        let metadata = PersistedMetadata {
            format_version: 2,
            key: table.key.clone(),
            state: TableState::Complete,
            generation: table.generation,
            answer_count: table.answers.len() as u64,
        };
        mutations.push(RawMutation::Put(
            metadata_key(&table.key).into_bytes(),
            bincode::serialize(&metadata).map_err(backend_error)?,
        ));
        self.storage
            .write_raw_batch(&mutations)
            .map_err(backend_error)
    }

    fn invalidate_all(&self) -> WamResult<()> {
        let metadata = self
            .storage
            .scan_prefix("table:meta:")
            .map_err(backend_error)?;
        let mut mutations = Vec::new();
        for (key, bytes) in metadata {
            let mut value: PersistedMetadata =
                bincode::deserialize(&bytes).map_err(backend_error)?;
            value.state = TableState::Stale;
            mutations.push(RawMutation::Put(
                key.into_bytes(),
                bincode::serialize(&value).map_err(backend_error)?,
            ));
        }
        self.storage
            .write_raw_batch(&mutations)
            .map_err(backend_error)
    }
}

fn answer_mutation(prefix: &str, index: usize, answer: &TableAnswer) -> WamResult<RawMutation> {
    Ok(RawMutation::Put(
        format!(
            "{}{index:020}:{fingerprint:016x}",
            prefix,
            fingerprint = answer.fingerprint
        )
        .into_bytes(),
        bincode::serialize(answer).map_err(backend_error)?,
    ))
}

fn metadata_key(key: &TableKey) -> String {
    format!("table:meta:{:016x}", key_fingerprint(key))
}

fn answer_prefix(key: &TableKey) -> String {
    format!("table:answer:{:016x}:", key_fingerprint(key))
}

fn key_fingerprint(key: &TableKey) -> u64 {
    fingerprint(&bincode::serialize(key).unwrap_or_default())
}

fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn backend_error(error: impl std::fmt::Display) -> WamError {
    table_error(&error.to_string())
}

fn table_error(message: &str) -> WamError {
    WamError::Provider(format!("persistent tabling: {message}"))
}
