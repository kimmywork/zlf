use serde::Serialize;
use zlf_storage::RawMutation;

use super::{TableDependencies, TableKey};
use crate::wam::error::{WamError, WamResult};
use crate::wam::fact_key::FactKey;
use crate::wam::predicate::PredicateKey;

#[derive(Clone, Copy)]
pub(crate) enum MutationKind {
    Put,
    Delete,
}

pub(crate) fn reverse_mutations(
    table: &TableKey,
    dependencies: &TableDependencies,
    kind: MutationKind,
) -> WamResult<Vec<RawMutation>> {
    let mut keys = dependencies
        .facts
        .iter()
        .map(|fact| reverse_key(&fact_reverse_prefix(fact), table))
        .collect::<Vec<_>>();
    keys.extend(
        dependencies
            .predicates
            .iter()
            .map(|predicate| reverse_key(&predicate_reverse_prefix(predicate), table))
            .collect::<Vec<_>>(),
    );
    keys.extend(
        dependencies
            .rules
            .iter()
            .map(|rule| reverse_key(&rule_reverse_prefix(rule), table)),
    );
    keys.extend(
        dependencies
            .tables
            .iter()
            .map(|dependency| reverse_key(&table_reverse_prefix(dependency), table)),
    );
    keys.into_iter()
        .map(|key| mutation(key, table, kind))
        .collect()
}

pub(crate) fn fact_reverse_prefix(fact: &FactKey) -> String {
    format!("table:revdep:fact:{:016x}:", serialized_fingerprint(fact))
}

pub(crate) fn predicate_reverse_prefix(predicate: &PredicateKey) -> String {
    format!(
        "table:revdep:predicate:{:016x}:",
        serialized_fingerprint(predicate)
    )
}

pub(crate) fn rule_reverse_prefix(rule: &str) -> String {
    format!("table:revdep:rule:{:016x}:", serialized_fingerprint(&rule))
}

pub(crate) fn table_reverse_prefix(table: &TableKey) -> String {
    format!("table:revdep:table:{:016x}:", serialized_fingerprint(table))
}

fn mutation(key: String, table: &TableKey, kind: MutationKind) -> WamResult<RawMutation> {
    match kind {
        MutationKind::Put => Ok(RawMutation::Put(
            key.into_bytes(),
            bincode::serialize(table).map_err(backend_error)?,
        )),
        MutationKind::Delete => Ok(RawMutation::Delete(key.into_bytes())),
    }
}

fn reverse_key(prefix: &str, table: &TableKey) -> String {
    format!("{}{:016x}", prefix, serialized_fingerprint(table))
}

fn serialized_fingerprint(value: &impl Serialize) -> u64 {
    bincode::serialize(value)
        .unwrap_or_default()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn backend_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(format!("persistent tabling: {error}"))
}
