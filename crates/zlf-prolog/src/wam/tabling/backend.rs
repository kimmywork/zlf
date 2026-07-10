use std::fmt::Debug;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use zlf_storage::{RawMutation, Storage};

use super::reverse::{
    fact_reverse_prefix, predicate_reverse_prefix, reverse_mutations, rule_reverse_prefix,
    table_reverse_prefix, MutationKind,
};
use super::{TableAnswer, TableDependencies, TableKey, TableState};
use crate::wam::error::{WamError, WamResult};
use crate::wam::fact_key::FactKey;
use crate::wam::predicate::PredicateKey;

#[derive(Debug, Clone)]
pub struct PersistedTable {
    pub key: TableKey,
    pub state: TableState,
    pub answers: Vec<TableAnswer>,
    pub generation: u64,
    pub dependencies: TableDependencies,
}

pub trait TableBackend: Debug + Send + Sync {
    fn load(&self, key: &TableKey) -> WamResult<Option<PersistedTable>>;
    fn store_complete(&self, table: &PersistedTable) -> WamResult<()>;
    fn invalidate_all(&self) -> WamResult<()>;
    fn invalidate_predicates(&self, predicates: &[PredicateKey]) -> WamResult<Vec<TableKey>>;
    fn invalidate_facts(&self, facts: &[FactKey]) -> WamResult<Vec<TableKey>>;
    fn invalidate_rules(&self, rules: &[String]) -> WamResult<Vec<TableKey>>;
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
        let Ok(mut metadata) = bincode::deserialize::<PersistedMetadata>(&bytes) else {
            return Ok(None);
        };
        if metadata.key != *key {
            return Err(table_error("persistent table metadata key mismatch"));
        }
        if metadata.format_version != 3 {
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

    fn reverse_tables(&self, prefix: &str) -> WamResult<Vec<TableKey>> {
        self.storage
            .scan_prefix(prefix)
            .map_err(backend_error)?
            .into_iter()
            .map(|(_, value)| bincode::deserialize(&value).map_err(backend_error))
            .collect()
    }

    fn propagate_and_mark(&self, mut pending: Vec<TableKey>) -> WamResult<Vec<TableKey>> {
        let mut impacted = std::collections::HashSet::new();
        while let Some(table) = pending.pop() {
            if impacted.insert(table.clone()) {
                pending.extend(self.reverse_tables(&table_reverse_prefix(&table))?);
            }
        }
        self.mark_stale(&impacted)?;
        Ok(impacted.into_iter().collect())
    }

    fn mark_stale(&self, tables: &std::collections::HashSet<TableKey>) -> WamResult<()> {
        let mut mutations = Vec::new();
        for table in tables {
            if let Some(mut metadata) = self.load_metadata(table)? {
                metadata.state = TableState::Stale;
                mutations.push(RawMutation::Put(
                    metadata_key(table).into_bytes(),
                    bincode::serialize(&metadata).map_err(backend_error)?,
                ));
            }
        }
        self.storage
            .write_raw_batch(&mutations)
            .map_err(backend_error)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedMetadata {
    format_version: u32,
    key: TableKey,
    state: TableState,
    generation: u64,
    answer_count: u64,
    dependencies: TableDependencies,
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
                dependencies: metadata.dependencies,
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
            dependencies: metadata.dependencies,
        }))
    }

    fn store_complete(&self, table: &PersistedTable) -> WamResult<()> {
        let mut mutations = self.answer_mutations(table)?;
        if let Some(old) = self.load_metadata(&table.key)? {
            mutations.extend(reverse_mutations(
                &table.key,
                &old.dependencies,
                MutationKind::Delete,
            )?);
        }
        mutations.extend(reverse_mutations(
            &table.key,
            &table.dependencies,
            MutationKind::Put,
        )?);
        let metadata = PersistedMetadata {
            format_version: 3,
            key: table.key.clone(),
            state: TableState::Complete,
            generation: table.generation,
            answer_count: table.answers.len() as u64,
            dependencies: table.dependencies.clone(),
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
            let Ok(mut value) = bincode::deserialize::<PersistedMetadata>(&bytes) else {
                continue;
            };
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

    fn invalidate_predicates(&self, predicates: &[PredicateKey]) -> WamResult<Vec<TableKey>> {
        let mut pending = Vec::new();
        for predicate in predicates {
            pending.extend(self.reverse_tables(&predicate_reverse_prefix(predicate))?);
        }
        self.propagate_and_mark(pending)
    }

    fn invalidate_facts(&self, facts: &[FactKey]) -> WamResult<Vec<TableKey>> {
        let mut pending = Vec::new();
        for fact in facts {
            pending.extend(self.reverse_tables(&fact_reverse_prefix(fact))?);
        }
        self.propagate_and_mark(pending)
    }

    fn invalidate_rules(&self, rules: &[String]) -> WamResult<Vec<TableKey>> {
        let mut pending = Vec::new();
        for rule in rules {
            pending.extend(self.reverse_tables(&rule_reverse_prefix(rule))?);
        }
        self.propagate_and_mark(pending)
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
    serialized_fingerprint(key)
}

fn serialized_fingerprint(value: &impl Serialize) -> u64 {
    fingerprint(&bincode::serialize(value).unwrap_or_default())
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
