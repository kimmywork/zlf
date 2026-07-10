use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::parser::Term;

use super::TableKey;
use crate::wam::fact_key::FactKey;
use crate::wam::predicate::PredicateKey;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDependencies {
    pub facts: HashSet<FactKey>,
    pub predicates: HashSet<PredicateKey>,
    pub tables: HashSet<TableKey>,
    pub rules: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableState {
    Evaluating,
    Complete,
    Stale,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableAnswer {
    pub values: Vec<Term>,
    pub fingerprint: u64,
}

#[derive(Debug, Clone)]
pub struct TableEntry {
    pub key: TableKey,
    pub state: TableState,
    pub answers: Vec<TableAnswer>,
    pub answer_set: HashSet<u64>,
    pub generation: u64,
    pub dependencies: TableDependencies,
}

impl TableEntry {
    pub fn new(key: TableKey, generation: u64) -> Self {
        Self {
            key,
            state: TableState::Evaluating,
            answers: Vec::new(),
            answer_set: HashSet::new(),
            generation,
            dependencies: TableDependencies::default(),
        }
    }

    pub fn insert(&mut self, values: Vec<Term>) -> bool {
        let fingerprint = fingerprint(&values);
        if !self.answer_set.insert(fingerprint) {
            return false;
        }
        self.answers.push(TableAnswer {
            values,
            fingerprint,
        });
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TableLimits {
    pub max_tables: usize,
    pub max_answers_per_table: usize,
    pub max_iterations: usize,
}

impl Default for TableLimits {
    fn default() -> Self {
        Self {
            max_tables: 10_000,
            max_answers_per_table: 1_000_000,
            max_iterations: 10_000,
        }
    }
}

#[derive(Debug, Default)]
pub struct TableStore {
    entries: HashMap<TableKey, TableEntry>,
    generation: u64,
    pub limits: TableLimits,
}

impl TableStore {
    pub fn with_limits(limits: TableLimits) -> Self {
        Self {
            entries: HashMap::new(),
            generation: 0,
            limits,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, key: &TableKey) -> Option<&TableEntry> {
        self.entries.get(key)
    }

    pub fn get_touch(&mut self, key: &TableKey) -> Option<TableEntry> {
        self.generation = self.generation.wrapping_add(1);
        let entry = self.entries.get_mut(key)?;
        entry.generation = self.generation;
        Some(entry.clone())
    }

    pub fn evict_oldest_complete(&mut self) -> bool {
        let key = self
            .entries
            .values()
            .filter(|entry| entry.state == TableState::Complete)
            .min_by_key(|entry| entry.generation)
            .map(|entry| entry.key.clone());
        key.is_some_and(|key| self.entries.remove(&key).is_some())
    }

    pub fn insert_entry(&mut self, mut entry: TableEntry) {
        self.generation = self.generation.wrapping_add(1);
        entry.generation = self.generation;
        self.entries.insert(entry.key.clone(), entry);
    }

    pub fn begin(&mut self, key: TableKey) -> &mut TableEntry {
        self.generation = self.generation.wrapping_add(1);
        self.entries
            .entry(key.clone())
            .or_insert_with(|| TableEntry::new(key, self.generation))
    }

    pub fn complete(&mut self, key: &TableKey) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.state = TableState::Complete;
        }
    }

    pub fn invalidate_predicates(&mut self, predicates: &HashSet<PredicateKey>) -> Vec<TableKey> {
        let impacted = self
            .entries
            .values()
            .filter(|entry| !entry.dependencies.predicates.is_disjoint(predicates))
            .map(|entry| entry.key.clone())
            .collect::<HashSet<_>>();
        self.invalidate_tables(impacted)
    }

    pub fn invalidate_facts(&mut self, facts: &HashSet<FactKey>) -> Vec<TableKey> {
        let impacted = self
            .entries
            .values()
            .filter(|entry| !entry.dependencies.facts.is_disjoint(facts))
            .map(|entry| entry.key.clone())
            .collect::<HashSet<_>>();
        self.invalidate_tables(impacted)
    }

    pub fn invalidate_rules(&mut self, rules: &HashSet<String>) -> Vec<TableKey> {
        let impacted = self
            .entries
            .values()
            .filter(|entry| !entry.dependencies.rules.is_disjoint(rules))
            .map(|entry| entry.key.clone())
            .collect::<HashSet<_>>();
        self.invalidate_tables(impacted)
    }

    fn invalidate_tables(&mut self, mut impacted: HashSet<TableKey>) -> Vec<TableKey> {
        loop {
            let before = impacted.len();
            let dependents = self
                .entries
                .values()
                .filter(|entry| !entry.dependencies.tables.is_disjoint(&impacted))
                .map(|entry| entry.key.clone())
                .collect::<Vec<_>>();
            impacted.extend(dependents);
            if impacted.len() == before {
                break;
            }
        }
        for key in &impacted {
            self.entries.remove(key);
        }
        impacted.into_iter().collect()
    }

    pub fn remove(&mut self, key: &TableKey) {
        self.entries.remove(key);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn fingerprint(values: &[Term]) -> u64 {
    bincode::serialize(values)
        .unwrap_or_default()
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}
