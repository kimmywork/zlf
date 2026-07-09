use std::collections::HashMap;

use crate::parser::{PrologRule, Term};

use super::error::WamResult;
use super::executor::WamExecutor;
use super::fact_provider::FactProvider;
use super::predicate::predicate_key;
use super::program_codegen::compile_query_program_with_rule_artifacts;
use super::rule_store::CompiledRuleArtifact;

#[derive(Debug, Default, Clone)]
pub struct WamRuntime {
    facts: Vec<Term>,
    rules: Vec<PrologRule>,
    compiled_rules: Vec<CompiledRuleArtifact>,
    register_count: usize,
}

impl WamRuntime {
    pub fn new(register_count: usize) -> Self {
        Self {
            facts: Vec::new(),
            rules: Vec::new(),
            compiled_rules: Vec::new(),
            register_count,
        }
    }

    pub fn add_fact(&mut self, fact: Term) {
        self.facts.push(fact);
    }

    pub fn add_rule(&mut self, rule: PrologRule) {
        self.rules.push(rule);
    }

    pub fn add_compiled_rule(&mut self, rule: CompiledRuleArtifact) {
        self.compiled_rules.push(rule);
    }

    pub fn query_all(&self, query: &Term) -> WamResult<Vec<HashMap<String, Term>>> {
        self.query_all_with_facts(query, self.facts.clone())
    }

    pub fn query_all_with_provider<P: FactProvider>(
        &self,
        query: &Term,
        provider: &P,
    ) -> WamResult<Vec<HashMap<String, Term>>> {
        let mut facts = self.facts.clone();
        for goal in provider_goals(query, &self.rules, &self.compiled_rules) {
            facts.extend(provider.facts_for_goal(&goal)?);
        }
        self.query_all_with_facts(query, facts)
    }

    fn query_all_with_facts(
        &self,
        query: &Term,
        facts: Vec<Term>,
    ) -> WamResult<Vec<HashMap<String, Term>>> {
        let compiled = compile_query_program_with_rule_artifacts(
            query,
            &facts,
            &self.rules,
            &self.compiled_rules,
        )?;
        let bindings = sorted_bindings(compiled.bindings);
        let registers = binding_registers(&bindings);
        let mut executor = WamExecutor::new(self.register_count);
        let rows = executor.execute_all_registers(&compiled.program, &registers)?;
        Ok(rows
            .into_iter()
            .map(|row| binding_row(&bindings, row))
            .collect())
    }
}

fn provider_goals(
    query: &Term,
    rules: &[PrologRule],
    artifacts: &[CompiledRuleArtifact],
) -> Vec<Term> {
    let mut goals = Vec::new();
    push_goal(&mut goals, query);
    for rule in rules {
        push_rule_goals(&mut goals, rule);
    }
    for artifact in artifacts {
        push_rule_goals(&mut goals, &artifact.source);
    }
    goals
}

fn push_rule_goals(goals: &mut Vec<Term>, rule: &PrologRule) {
    push_goal(goals, &rule.head);
    for goal in &rule.body {
        push_goal(goals, goal);
    }
}

fn push_goal(goals: &mut Vec<Term>, term: &Term) {
    let Some(key) = predicate_key(term) else {
        return;
    };
    if !goals
        .iter()
        .any(|goal| predicate_key(goal) == Some(key.clone()))
    {
        goals.push(term.clone());
    }
}

fn sorted_bindings(bindings: HashMap<String, usize>) -> Vec<(String, usize)> {
    let mut bindings: Vec<_> = bindings
        .into_iter()
        .filter(|(name, _)| name != "_")
        .collect();
    bindings.sort_by(|left, right| left.0.cmp(&right.0));
    bindings
}

fn binding_registers(bindings: &[(String, usize)]) -> Vec<usize> {
    bindings.iter().map(|(_, register)| *register).collect()
}

fn binding_row(bindings: &[(String, usize)], row: Vec<Term>) -> HashMap<String, Term> {
    bindings
        .iter()
        .map(|(name, _)| name.clone())
        .zip(row)
        .collect()
}
