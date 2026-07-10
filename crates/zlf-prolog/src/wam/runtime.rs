use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use crate::parser::{PrologRule, Term};

use super::error::WamResult;
use super::executor::WamExecutor;
use super::fact_provider::FactProvider;
use super::predicate::predicate_key;
use super::program_codegen::compile_query_program_with_rule_artifacts;
use super::proof::ProofAnswer;
use super::rule_store::CompiledRuleArtifact;
use super::stdlib_rules::core_library_rules;
use super::tabling::{evaluate_tabled, TableKey, TableState, TableStore};
use zlf_storage::Storage;

#[derive(Debug)]
pub struct WamRuntime {
    pub(crate) facts: Vec<Term>,
    pub(crate) rules: Vec<PrologRule>,
    pub(crate) compiled_rules: Vec<CompiledRuleArtifact>,
    pub(crate) register_count: usize,
    pub(crate) system_rule_count: usize,
    pub(crate) tabled: HashSet<super::PredicateKey>,
    pub(crate) tables: RwLock<TableStore>,
}

impl Default for WamRuntime {
    fn default() -> Self {
        Self::new(64)
    }
}

impl WamRuntime {
    pub fn new(register_count: usize) -> Self {
        let rules = core_library_rules();
        let system_rule_count = rules.len();
        Self {
            facts: Vec::new(),
            rules,
            compiled_rules: Vec::new(),
            register_count,
            system_rule_count,
            tabled: HashSet::new(),
            tables: RwLock::new(TableStore::default()),
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

    pub fn declare_tabled(&mut self, key: super::PredicateKey) {
        self.tabled.insert(key);
    }

    pub fn is_tabled(&self, key: &super::PredicateKey) -> bool {
        self.tabled.contains(key)
    }

    pub fn table_state(&self, key: &TableKey) -> Option<TableState> {
        self.tables
            .read()
            .ok()
            .and_then(|tables| tables.get(key).map(|entry| entry.state))
    }

    pub fn query_all(&self, query: &Term) -> WamResult<Vec<HashMap<String, Term>>> {
        if predicate_key(query).is_some_and(|key| self.is_tabled(&key)) {
            let provider = super::StaticFactProvider::default();
            return evaluate_tabled(self, query, &provider, None);
        }
        self.query_all_with_facts(query, self.facts.clone(), None)
    }

    pub fn query_all_with_proof(&self, query: &Term) -> WamResult<Vec<ProofAnswer>> {
        self.query_all_with_facts_and_proof(query, self.facts.clone(), None)
    }

    pub fn query_all_with_provider<P: FactProvider>(
        &self,
        query: &Term,
        provider: &P,
    ) -> WamResult<Vec<HashMap<String, Term>>> {
        self.query_all_with_provider_and_optional_storage(query, provider, None)
    }

    pub fn query_all_with_provider_and_storage<P: FactProvider>(
        &self,
        query: &Term,
        provider: &P,
        storage: &Storage,
    ) -> WamResult<Vec<HashMap<String, Term>>> {
        self.query_all_with_provider_and_optional_storage(query, provider, Some(storage))
    }

    pub fn query_all_with_provider_and_storage_with_proof(
        &self,
        query: &Term,
        provider: &dyn FactProvider,
        storage: &Storage,
    ) -> WamResult<Vec<ProofAnswer>> {
        let mut facts = self.facts.clone();
        for goal in provider_goals(query, &self.rules, &self.compiled_rules) {
            facts.extend(provider.facts_for_goal(&goal)?);
        }
        self.query_all_with_facts_and_proof(query, facts, Some(storage))
    }

    pub fn query_all_with_provider_and_optional_storage(
        &self,
        query: &Term,
        provider: &dyn FactProvider,
        storage: Option<&Storage>,
    ) -> WamResult<Vec<HashMap<String, Term>>> {
        if predicate_key(query).is_some_and(|key| self.is_tabled(&key)) {
            return evaluate_tabled(self, query, provider, storage);
        }
        let mut facts = self.facts.clone();
        for goal in provider_goals(query, &self.rules, &self.compiled_rules) {
            facts.extend(provider.facts_for_goal(&goal)?);
        }
        self.query_all_with_facts(query, facts, storage)
    }

    fn query_all_with_facts(
        &self,
        query: &Term,
        facts: Vec<Term>,
        storage: Option<&Storage>,
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
        let rows =
            executor.execute_all_registers_with_storage(&compiled.program, &registers, storage)?;
        Ok(rows
            .into_iter()
            .map(|row| binding_row(&bindings, row))
            .collect())
    }

    fn query_all_with_facts_and_proof(
        &self,
        query: &Term,
        facts: Vec<Term>,
        storage: Option<&Storage>,
    ) -> WamResult<Vec<ProofAnswer>> {
        let compiled = compile_query_program_with_rule_artifacts(
            query,
            &facts,
            &self.rules,
            &self.compiled_rules,
        )?;
        let bindings = sorted_bindings(compiled.bindings);
        let registers = binding_registers(&bindings);
        let mut executor = WamExecutor::new(self.register_count);
        Ok(executor
            .execute_all_registers_with_proof(&compiled.program, &registers, storage)?
            .into_iter()
            .map(|(row, proof)| ProofAnswer {
                bindings: binding_row(&bindings, row),
                proof,
            })
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
    push_meta_goals(goals, term);
}

fn push_meta_goals(goals: &mut Vec<Term>, term: &Term) {
    let Term::Compound { name, args } = term else {
        return;
    };
    match (name.as_str(), args.as_slice()) {
        ("call", [closure, extras @ ..]) => {
            if let Some(goal) = expand_callable(closure, extras) {
                push_goal(goals, &goal);
            }
        }
        ("once" | "\\+", [goal]) => push_goal(goals, goal),
        (";" | "->", [left, right]) => {
            push_goal(goals, left);
            push_goal(goals, right);
        }
        _ => {}
    }
}

fn expand_callable(closure: &Term, extras: &[Term]) -> Option<Term> {
    match closure {
        Term::Atom(name) => Some(Term::Compound {
            name: name.clone(),
            args: extras.to_vec(),
        }),
        Term::Compound { name, args } => {
            let mut combined = args.clone();
            combined.extend_from_slice(extras);
            Some(Term::Compound {
                name: name.clone(),
                args: combined,
            })
        }
        _ => None,
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
