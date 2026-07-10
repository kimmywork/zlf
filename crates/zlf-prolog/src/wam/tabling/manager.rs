use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use serde::Serialize;

use super::backend::{PersistedTable, TableBackend};
use super::{TableDependencies, TableEntry, TableKey, TableLimits, TableState, TableStore};
use crate::parser::Term;
use crate::wam::error::{WamError, WamResult};
use crate::wam::fact_key::FactKey;
use crate::wam::predicate::PredicateKey;

#[derive(Debug, Default)]
struct TableMetrics {
    hot_hits: AtomicU64,
    persistent_hits: AtomicU64,
    misses: AtomicU64,
    tables_completed: AtomicU64,
    stale_invalidations: AtomicU64,
    iterations: AtomicU64,
    inserted_answers: AtomicU64,
    duplicate_answers: AtomicU64,
    evictions: AtomicU64,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct TableMetricsSnapshot {
    pub hot_hits: u64,
    pub persistent_hits: u64,
    pub misses: u64,
    pub tables_completed: u64,
    pub stale_invalidations: u64,
    pub iterations: u64,
    pub inserted_answers: u64,
    pub duplicate_answers: u64,
    pub evictions: u64,
}

#[derive(Debug)]
pub struct TableManager {
    hot: RwLock<TableStore>,
    backend: Option<Arc<dyn TableBackend>>,
    metrics: TableMetrics,
}

impl Default for TableManager {
    fn default() -> Self {
        Self::memory()
    }
}

impl TableManager {
    pub fn memory() -> Self {
        Self {
            hot: RwLock::new(TableStore::default()),
            backend: None,
            metrics: TableMetrics::default(),
        }
    }

    pub fn with_backend(backend: Arc<dyn TableBackend>) -> Self {
        Self::with_backend_and_limits(backend, TableLimits::default())
    }

    pub fn with_backend_and_limits(backend: Arc<dyn TableBackend>, limits: TableLimits) -> Self {
        Self {
            hot: RwLock::new(TableStore::with_limits(limits)),
            backend: Some(backend),
            metrics: TableMetrics::default(),
        }
    }

    pub fn lookup(&self, key: &TableKey) -> WamResult<Option<TableEntry>> {
        if let Some(entry) = self.hot.write().map_err(lock_error)?.get_touch(key) {
            if entry.state == TableState::Complete {
                self.metrics.hot_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(Some(entry));
            }
        }
        if let Some(table) = self
            .backend
            .as_ref()
            .map(|backend| backend.load(key))
            .transpose()?
            .flatten()
        {
            if table.state == TableState::Complete {
                let entry = restored_entry(table);
                self.hot
                    .write()
                    .map_err(lock_error)?
                    .insert_entry(entry.clone());
                self.metrics.persistent_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(Some(entry));
            }
        }
        self.metrics.misses.fetch_add(1, Ordering::Relaxed);
        Ok(None)
    }

    pub fn begin(&self, key: TableKey) -> WamResult<()> {
        let mut hot = self.hot.write().map_err(lock_error)?;
        if hot.get(&key).is_none() && hot.len() >= hot.limits.max_tables {
            if !hot.evict_oldest_complete() {
                return Err(table_error("maximum active tables exceeded"));
            }
            self.metrics.evictions.fetch_add(1, Ordering::Relaxed);
        }
        hot.begin(key).state = TableState::Evaluating;
        Ok(())
    }

    pub fn complete(
        &self,
        key: &TableKey,
        answers: Vec<Vec<Term>>,
        dependencies: TableDependencies,
    ) -> WamResult<()> {
        let persisted = {
            let mut hot = self.hot.write().map_err(lock_error)?;
            let entry = hot.begin(key.clone());
            entry.answers.clear();
            entry.answer_set.clear();
            for answer in answers {
                entry.insert(answer);
            }
            entry.state = TableState::Complete;
            entry.dependencies = dependencies.clone();
            PersistedTable {
                key: key.clone(),
                state: TableState::Complete,
                answers: entry.answers.clone(),
                generation: entry.generation,
                dependencies,
            }
        };
        if let Some(backend) = &self.backend {
            backend.store_complete(&persisted)?;
        }
        self.metrics
            .tables_completed
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn state(&self, key: &TableKey) -> WamResult<Option<TableState>> {
        if let Some(entry) = self.hot.read().map_err(lock_error)?.get(key) {
            return Ok(Some(entry.state));
        }
        Ok(self
            .backend
            .as_ref()
            .map(|backend| backend.load(key))
            .transpose()?
            .flatten()
            .map(|table| table.state))
    }

    pub fn limits(&self) -> WamResult<TableLimits> {
        Ok(self.hot.read().map_err(lock_error)?.limits)
    }

    pub fn invalidate_predicates(&self, predicates: &[PredicateKey]) -> WamResult<Vec<TableKey>> {
        let predicates = predicates.iter().cloned().collect::<HashSet<_>>();
        let mut impacted = self
            .hot
            .write()
            .map_err(lock_error)?
            .invalidate_predicates(&predicates)
            .into_iter()
            .collect::<HashSet<_>>();
        if let Some(backend) = &self.backend {
            impacted.extend(
                backend.invalidate_predicates(&predicates.into_iter().collect::<Vec<_>>())?,
            );
        }
        self.metrics
            .stale_invalidations
            .fetch_add(impacted.len() as u64, Ordering::Relaxed);
        Ok(impacted.into_iter().collect())
    }

    pub fn invalidate_facts(&self, facts: &[FactKey]) -> WamResult<Vec<TableKey>> {
        let facts = facts.iter().cloned().collect::<HashSet<_>>();
        let mut impacted = self
            .hot
            .write()
            .map_err(lock_error)?
            .invalidate_facts(&facts)
            .into_iter()
            .collect::<HashSet<_>>();
        if let Some(backend) = &self.backend {
            impacted.extend(backend.invalidate_facts(&facts.into_iter().collect::<Vec<_>>())?);
        }
        self.metrics
            .stale_invalidations
            .fetch_add(impacted.len() as u64, Ordering::Relaxed);
        Ok(impacted.into_iter().collect())
    }

    pub fn invalidate_rules(&self, rules: &[String]) -> WamResult<Vec<TableKey>> {
        let rules = rules.iter().cloned().collect::<HashSet<_>>();
        let mut impacted = self
            .hot
            .write()
            .map_err(lock_error)?
            .invalidate_rules(&rules)
            .into_iter()
            .collect::<HashSet<_>>();
        if let Some(backend) = &self.backend {
            impacted.extend(backend.invalidate_rules(&rules.into_iter().collect::<Vec<_>>())?);
        }
        self.metrics
            .stale_invalidations
            .fetch_add(impacted.len() as u64, Ordering::Relaxed);
        Ok(impacted.into_iter().collect())
    }

    pub fn invalidate_all(&self) -> WamResult<()> {
        self.hot.write().map_err(lock_error)?.clear();
        if let Some(backend) = &self.backend {
            backend.invalidate_all()?;
        }
        self.metrics
            .stale_invalidations
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn record_iteration(&self) {
        self.metrics.iterations.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_answers(&self, inserted: usize, duplicates: usize) {
        self.metrics
            .inserted_answers
            .fetch_add(inserted as u64, Ordering::Relaxed);
        self.metrics
            .duplicate_answers
            .fetch_add(duplicates as u64, Ordering::Relaxed);
    }

    pub fn metrics(&self) -> TableMetricsSnapshot {
        TableMetricsSnapshot {
            hot_hits: self.metrics.hot_hits.load(Ordering::Relaxed),
            persistent_hits: self.metrics.persistent_hits.load(Ordering::Relaxed),
            misses: self.metrics.misses.load(Ordering::Relaxed),
            tables_completed: self.metrics.tables_completed.load(Ordering::Relaxed),
            stale_invalidations: self.metrics.stale_invalidations.load(Ordering::Relaxed),
            iterations: self.metrics.iterations.load(Ordering::Relaxed),
            inserted_answers: self.metrics.inserted_answers.load(Ordering::Relaxed),
            duplicate_answers: self.metrics.duplicate_answers.load(Ordering::Relaxed),
            evictions: self.metrics.evictions.load(Ordering::Relaxed),
        }
    }
}

fn restored_entry(table: PersistedTable) -> TableEntry {
    TableEntry {
        key: table.key,
        state: table.state,
        answer_set: table
            .answers
            .iter()
            .map(|answer| answer.fingerprint)
            .collect::<HashSet<_>>(),
        answers: table.answers,
        generation: table.generation,
        dependencies: table.dependencies,
    }
}

fn lock_error(error: impl std::fmt::Display) -> WamError {
    table_error(&error.to_string())
}

fn table_error(message: &str) -> WamError {
    WamError::Provider(format!("tabling: {message}"))
}
