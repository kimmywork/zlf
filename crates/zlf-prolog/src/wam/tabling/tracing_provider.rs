use std::collections::HashSet;
use std::sync::Mutex;

use crate::parser::Term;
use crate::wam::error::{WamError, WamResult};
use crate::wam::fact_key::{term_to_fact_key, FactKey};
use crate::wam::fact_provider::FactProvider;
use crate::wam::predicate::PredicateKey;

pub(crate) struct DependencyProvider<'a> {
    inner: &'a dyn FactProvider,
    facts: Mutex<HashSet<FactKey>>,
}

impl<'a> DependencyProvider<'a> {
    pub(crate) fn new(inner: &'a dyn FactProvider) -> Self {
        Self {
            inner,
            facts: Mutex::new(HashSet::new()),
        }
    }

    pub(crate) fn facts(&self) -> WamResult<HashSet<FactKey>> {
        self.facts
            .lock()
            .map(|facts| facts.clone())
            .map_err(lock_error)
    }

    fn record(&self, facts: &[Term]) -> WamResult<()> {
        self.facts
            .lock()
            .map_err(lock_error)?
            .extend(facts.iter().filter_map(term_to_fact_key));
        Ok(())
    }
}

impl FactProvider for DependencyProvider<'_> {
    fn facts_for(&self, key: &PredicateKey) -> WamResult<Vec<Term>> {
        let facts = self.inner.facts_for(key)?;
        self.record(&facts)?;
        Ok(facts)
    }

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        let facts = self.inner.facts_for_goal(goal)?;
        self.record(&facts)?;
        Ok(facts)
    }
}

fn lock_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(format!("table dependency lock: {error}"))
}
