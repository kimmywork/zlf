use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::parser::{PrologRule, Term};

use super::error::WamResult;
use super::executor::WamExecutor;
use super::fact_key::term_to_fact_key;
use super::predicate::{predicate_key, PredicateKey};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofKind {
    Fact,
    Rule,
    Builtin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofClause {
    pub id: String,
    pub predicate: PredicateKey,
    pub kind: ProofKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofNode {
    pub clause: ProofClause,
    pub parent: Option<usize>,
    pub substitutions: Vec<Term>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProofTree {
    pub nodes: Vec<ProofNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofAnswer {
    pub bindings: HashMap<String, Term>,
    pub proof: ProofTree,
}

#[derive(Debug, Default)]
pub(crate) struct ProofState {
    enabled: bool,
    nodes: Vec<ProofNode>,
    active: Vec<(usize, usize)>,
}

impl WamExecutor {
    pub(crate) fn enter_proof(&mut self, clause: &ProofClause) -> WamResult<bool> {
        if self.proof.is_enabled() {
            let substitutions = (0..clause.predicate.arity)
                .map(|register| self.register_term(register))
                .collect::<WamResult<Vec<_>>>()?;
            self.proof
                .enter(clause, self.call_stack.len(), substitutions);
        }
        Ok(true)
    }
}

impl ProofState {
    pub(crate) fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    pub(crate) fn reset(&mut self) {
        self.nodes.clear();
        self.active.clear();
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn enter(&mut self, clause: &ProofClause, depth: usize, substitutions: Vec<Term>) {
        if !self.enabled {
            return;
        }
        let index = self.nodes.len();
        self.nodes.push(ProofNode {
            clause: clause.clone(),
            parent: self.active.last().map(|(parent, _)| *parent),
            substitutions,
        });
        self.active.push((index, depth));
    }

    pub(crate) fn record_leaf(&mut self, clause: ProofClause, substitutions: Vec<Term>) {
        if self.enabled {
            self.nodes.push(ProofNode {
                clause,
                parent: self.active.last().map(|(parent, _)| *parent),
                substitutions,
            });
        }
    }

    pub(crate) fn complete_depth(&mut self, depth: usize) {
        if self.enabled {
            while self
                .active
                .last()
                .is_some_and(|(_, active_depth)| *active_depth >= depth)
            {
                self.active.pop();
            }
        }
    }

    pub(crate) fn checkpoint(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn restore(&mut self, checkpoint: usize) {
        if self.enabled {
            self.nodes.truncate(checkpoint);
            self.active.retain(|(index, _)| *index < checkpoint);
        }
    }

    pub(crate) fn snapshot(&self) -> ProofTree {
        ProofTree {
            nodes: self.nodes.clone(),
        }
    }
}

pub(crate) fn fact_clause(term: &Term) -> Option<ProofClause> {
    let id = term_to_fact_key(term)
        .map(|key| stable_id("fact", &key))
        .unwrap_or_else(|| stable_id("fact", term));
    Some(ProofClause {
        id,
        predicate: predicate_key(term)?,
        kind: ProofKind::Fact,
    })
}

pub(crate) fn rule_clause(rule: &PrologRule) -> Option<ProofClause> {
    rule_clause_with_id(rule, stable_rule_id(rule))
}

pub(crate) fn rule_clause_with_id(rule: &PrologRule, id: String) -> Option<ProofClause> {
    Some(ProofClause {
        id,
        predicate: predicate_key(&rule.head)?,
        kind: ProofKind::Rule,
    })
}

pub(crate) fn builtin_clause(key: &PredicateKey) -> ProofClause {
    ProofClause {
        id: format!("builtin:{}/{}", key.name, key.arity),
        predicate: key.clone(),
        kind: ProofKind::Builtin,
    }
}

pub(crate) fn stable_rule_id(rule: &PrologRule) -> String {
    stable_id("rule", rule)
}

fn stable_id(prefix: &str, value: &impl Serialize) -> String {
    let bytes = bincode::serialize(value).unwrap_or_default();
    let hash = bytes
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    format!("{prefix}:{hash:016x}")
}
