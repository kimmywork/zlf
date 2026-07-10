use std::collections::HashSet;

use crate::parser::{PrologRule, Term};
use crate::wam::error::{WamError, WamResult};
use crate::wam::fact_provider::FactProvider;
use crate::wam::predicate::PredicateKey;
use crate::wam::runtime::WamRuntime;
use zlf_storage::Storage;

use super::delta::{insert_new_answers, is_recursive, renamed_facts, rule_variants};
use super::evaluator::evaluate_rule;

pub(crate) fn run_fixpoint(
    runtime: &WamRuntime,
    rules: &[PrologRule],
    component: &HashSet<PredicateKey>,
    facts: Vec<Term>,
    provider: &dyn FactProvider,
    storage: Option<&Storage>,
    limits: super::TableLimits,
) -> WamResult<Vec<Term>> {
    let fingerprints = facts.iter().map(fingerprint).collect::<HashSet<_>>();
    let mut state = Fixpoint {
        runtime,
        rules,
        component,
        provider,
        storage,
        maximum_answers: limits.max_answers_per_table,
        facts,
        fingerprints,
    };
    let delta = state.seed_base()?;
    state.run(delta, limits.max_iterations)
}

struct Fixpoint<'a> {
    runtime: &'a WamRuntime,
    rules: &'a [PrologRule],
    component: &'a HashSet<PredicateKey>,
    provider: &'a dyn FactProvider,
    storage: Option<&'a Storage>,
    maximum_answers: usize,
    facts: Vec<Term>,
    fingerprints: HashSet<u64>,
}

impl Fixpoint<'_> {
    fn seed_base(&mut self) -> WamResult<Vec<Term>> {
        let mut delta = renamed_facts(&self.facts, self.component);
        for rule in self
            .rules
            .iter()
            .filter(|rule| !is_recursive(rule, self.component))
        {
            let answers =
                evaluate_rule(self.runtime, rule, &self.facts, self.provider, self.storage)?;
            let inserted = self.insert(answers)?;
            delta.extend(renamed_facts(&inserted, self.component));
        }
        Ok(delta)
    }

    fn run(mut self, mut delta: Vec<Term>, maximum_iterations: usize) -> WamResult<Vec<Term>> {
        for _ in 0..maximum_iterations {
            if delta.is_empty() {
                return Ok(self.facts);
            }
            self.runtime.table_manager.record_iteration();
            delta = self.iterate(delta)?;
        }
        Err(table_error("maximum table iterations exceeded"))
    }

    fn iterate(&mut self, delta: Vec<Term>) -> WamResult<Vec<Term>> {
        let mut evaluation_facts = self.facts.clone();
        evaluation_facts.extend(delta);
        let mut inserted = Vec::new();
        for rule in self
            .rules
            .iter()
            .filter(|rule| is_recursive(rule, self.component))
        {
            for variant in rule_variants(rule, self.component) {
                let answers = evaluate_rule(
                    self.runtime,
                    &variant,
                    &evaluation_facts,
                    self.provider,
                    self.storage,
                )?;
                inserted.extend(self.insert(answers)?);
            }
        }
        Ok(renamed_facts(&inserted, self.component))
    }

    fn insert(&mut self, answers: Vec<Term>) -> WamResult<Vec<Term>> {
        let candidates = answers.len();
        let inserted = insert_new_answers(
            &mut self.facts,
            &mut self.fingerprints,
            answers,
            self.maximum_answers,
        )?;
        self.runtime
            .table_manager
            .record_answers(inserted.len(), candidates.saturating_sub(inserted.len()));
        Ok(inserted)
    }
}

fn fingerprint(term: &Term) -> u64 {
    bincode::serialize(term)
        .unwrap_or_default()
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn table_error(message: &str) -> WamError {
    WamError::Provider(format!("tabling: {message}"))
}
