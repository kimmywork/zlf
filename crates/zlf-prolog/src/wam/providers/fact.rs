use crate::parser::Term;

use super::error::WamResult;
use super::predicate::{predicate_key, PredicateKey};

pub trait FactProvider {
    fn facts_for(&self, key: &PredicateKey) -> WamResult<Vec<Term>>;

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        predicate_key(goal).map_or_else(|| Ok(Vec::new()), |key| self.facts_for(&key))
    }
}

#[derive(Debug, Clone, Default)]
pub struct StaticFactProvider {
    facts: Vec<Term>,
}

impl StaticFactProvider {
    pub fn new(facts: Vec<Term>) -> Self {
        Self { facts }
    }
}

impl FactProvider for StaticFactProvider {
    fn facts_for(&self, key: &PredicateKey) -> WamResult<Vec<Term>> {
        Ok(self
            .facts
            .iter()
            .filter(|fact| super::predicate_key(fact).as_ref() == Some(key))
            .cloned()
            .collect())
    }
}
