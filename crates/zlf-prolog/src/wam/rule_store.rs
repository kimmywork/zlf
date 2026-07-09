use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::parser::{PrologRule, Term};

use super::codegen::WamCodegen;
use super::error::{WamError, WamResult};
use super::instruction::Instruction;
use super::predicate::{compound_args, predicate_key, PredicateKey};
use super::program::WamProgram;
use zlf_storage::Storage;

const PREFIX: &str = "rule:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledRuleArtifact {
    pub key: PredicateKey,
    pub source: PrologRule,
    pub temp_base: usize,
    pub program: WamProgram,
}

impl CompiledRuleArtifact {
    pub fn compile(rule: &PrologRule) -> WamResult<Self> {
        let key = predicate_key(&rule.head).ok_or(WamError::ExpectedFunctor(0))?;
        let temp_base = max_goal_arity(rule);
        Ok(Self {
            key,
            source: rule.clone(),
            temp_base,
            program: WamCodegen::compile_rule_clause_with_temp_start(rule, temp_base)?,
        })
    }

    pub fn materialize(&self, temp_start: usize) -> WamProgram {
        let target_base = temp_start.max(self.temp_base);
        let delta = target_base.saturating_sub(self.temp_base);
        WamProgram::new(
            self.program
                .instructions()
                .iter()
                .map(|instruction| relocate_instruction(instruction, self.temp_base, delta))
                .collect(),
        )
    }
}

pub struct StorageRuleStore<'a> {
    storage: &'a Storage,
}

impl<'a> StorageRuleStore<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn add_rule(&self, rule: &PrologRule) -> WamResult<String> {
        self.add_compiled_rule(&CompiledRuleArtifact::compile(rule)?)
    }

    pub fn add_compiled_rule(&self, artifact: &CompiledRuleArtifact) -> WamResult<String> {
        let key = rule_key(&artifact.key);
        let data = bincode::serialize(artifact).map_err(provider_error)?;
        self.storage.put_raw(&key, &data).map_err(provider_error)?;
        Ok(key)
    }

    pub fn rules_for(&self, key: &PredicateKey) -> WamResult<Vec<CompiledRuleArtifact>> {
        self.storage
            .scan_prefix(&rule_prefix(key))
            .map_err(provider_error)?
            .into_iter()
            .map(|(_, value)| bincode::deserialize(&value).map_err(provider_error))
            .collect()
    }

    pub fn all_rules(&self) -> WamResult<Vec<CompiledRuleArtifact>> {
        self.storage
            .scan_prefix(PREFIX)
            .map_err(provider_error)?
            .into_iter()
            .map(|(_, value)| bincode::deserialize(&value).map_err(provider_error))
            .collect()
    }
}

fn max_goal_arity(rule: &PrologRule) -> usize {
    std::iter::once(&rule.head)
        .chain(rule.body.iter())
        .filter_map(compound_args)
        .map(<[Term]>::len)
        .max()
        .unwrap_or_default()
}

#[allow(clippy::too_many_lines)]
fn relocate_instruction(instruction: &Instruction, base: usize, delta: usize) -> Instruction {
    use Instruction::*;
    match instruction {
        PutVariable { register } => PutVariable {
            register: relocate(*register, base, delta),
        },
        PutValue { source, target } => PutValue {
            source: relocate(*source, base, delta),
            target: relocate(*target, base, delta),
        },
        PutPermanentValue { slot, register } => PutPermanentValue {
            slot: *slot,
            register: relocate(*register, base, delta),
        },
        PutConstant { value, register } => PutConstant {
            value: value.clone(),
            register: relocate(*register, base, delta),
        },
        PutStructure {
            name,
            arity,
            register,
        } => PutStructure {
            name: name.clone(),
            arity: *arity,
            register: relocate(*register, base, delta),
        },
        PutList { arity, register } => PutList {
            arity: *arity,
            register: relocate(*register, base, delta),
        },
        SetVariable { register } => SetVariable {
            register: relocate(*register, base, delta),
        },
        SetValue { register } => SetValue {
            register: relocate(*register, base, delta),
        },
        SetPermanentValue { slot } => SetPermanentValue { slot: *slot },
        GetConstant { value, register } => GetConstant {
            value: value.clone(),
            register: relocate(*register, base, delta),
        },
        GetStructure {
            name,
            arity,
            register,
        } => GetStructure {
            name: name.clone(),
            arity: *arity,
            register: relocate(*register, base, delta),
        },
        GetList { arity, register } => GetList {
            arity: *arity,
            register: relocate(*register, base, delta),
        },
        GetValue { left, right } => GetValue {
            left: relocate(*left, base, delta),
            right: relocate(*right, base, delta),
        },
        GetPermanentValue { slot, register } => GetPermanentValue {
            slot: *slot,
            register: relocate(*register, base, delta),
        },
        UnifyVariable { register } => UnifyVariable {
            register: relocate(*register, base, delta),
        },
        UnifyValue { register } => UnifyValue {
            register: relocate(*register, base, delta),
        },
        UnifyPermanentValue { slot } => UnifyPermanentValue { slot: *slot },
        UnifyRegisters { left, right } => UnifyRegisters {
            left: relocate(*left, base, delta),
            right: relocate(*right, base, delta),
        },
        SwitchOnTerm {
            register,
            variable,
            constant,
            list,
            structure,
        } => SwitchOnTerm {
            register: relocate(*register, base, delta),
            variable: *variable,
            constant: *constant,
            list: *list,
            structure: *structure,
        },
        SwitchOnConstant {
            register,
            cases,
            default,
        } => SwitchOnConstant {
            register: relocate(*register, base, delta),
            cases: cases.clone(),
            default: *default,
        },
        SwitchOnStructure {
            register,
            cases,
            default,
        } => SwitchOnStructure {
            register: relocate(*register, base, delta),
            cases: cases.clone(),
            default: *default,
        },
        _ => instruction.clone(),
    }
}

fn relocate(register: usize, base: usize, delta: usize) -> usize {
    if register >= base {
        register + delta
    } else {
        register
    }
}

fn rule_key(key: &PredicateKey) -> String {
    format!("{}{}/{}:{}", PREFIX, key.name, key.arity, next_id())
}

fn rule_prefix(key: &PredicateKey) -> String {
    format!("{}{}/{}:", PREFIX, key.name, key.arity)
}

fn next_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{nanos}")
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
