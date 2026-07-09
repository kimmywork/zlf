use crate::parser::Term;

use super::error::WamResult;
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;

#[derive(Default)]
pub struct CompositeFactProvider<'a> {
    providers: Vec<&'a dyn FactProvider>,
}

impl<'a> CompositeFactProvider<'a> {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn with(mut self, provider: &'a dyn FactProvider) -> Self {
        self.providers.push(provider);
        self
    }
}

impl FactProvider for CompositeFactProvider<'_> {
    fn facts_for(&self, key: &PredicateKey) -> WamResult<Vec<Term>> {
        let mut facts = Vec::new();
        for provider in &self.providers {
            facts.extend(provider.facts_for(key)?);
        }
        Ok(facts)
    }

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        let mut facts = Vec::new();
        for provider in &self.providers {
            facts.extend(provider.facts_for_goal(goal)?);
        }
        Ok(facts)
    }
}
