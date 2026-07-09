use std::collections::HashMap;

use crate::parser::{PrologRule, Term};

use super::error::{WamError, WamResult};
use super::m1::{compile_fact, predicate_key, CompiledFact, M1Machine, PredicateKey};
use super::variable_analysis::permanent_variables;

#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub key: PredicateKey,
    pub head: Term,
    pub body: Vec<Term>,
    pub permanent_vars: Vec<String>,
}

#[derive(Debug)]
pub struct M2Machine {
    register_count: usize,
}

impl M2Machine {
    pub fn new(register_count: usize) -> Self {
        Self { register_count }
    }

    pub fn solve_rule(
        &self,
        goal: &Term,
        rule: &CompiledRule,
        facts: &[CompiledFact],
    ) -> WamResult<Vec<HashMap<String, Term>>> {
        if predicate_key(goal) != Some(rule.key.clone()) {
            return Ok(Vec::new());
        }
        let mut bindings = HashMap::new();
        if !unify_terms(goal, &rule.head, &mut bindings) {
            return Ok(Vec::new());
        }
        self.solve_goals(&rule.body, bindings, facts)
    }

    fn solve_goals(
        &self,
        goals: &[Term],
        bindings: HashMap<String, Term>,
        facts: &[CompiledFact],
    ) -> WamResult<Vec<HashMap<String, Term>>> {
        let Some((first, rest)) = goals.split_first() else {
            return Ok(vec![normalize(bindings)]);
        };
        let goal = substitute(first, &bindings);
        let mut results = Vec::new();
        for solution in self.solve_fact_goal(&goal, facts)? {
            results.extend(self.solve_goals(rest, merge(&bindings, solution), facts)?);
        }
        Ok(results)
    }

    fn solve_fact_goal(
        &self,
        goal: &Term,
        facts: &[CompiledFact],
    ) -> WamResult<Vec<HashMap<String, Term>>> {
        let mut results = Vec::new();
        for fact in facts {
            let mut machine = M1Machine::new(self.register_count);
            if let Some(solution) = machine.solve_fact(goal, fact)? {
                results.push(solution);
            }
        }
        Ok(results)
    }
}

pub fn compile_rule(rule: PrologRule) -> WamResult<CompiledRule> {
    let key = predicate_key(&rule.head).ok_or(WamError::ExpectedFunctor(0))?;
    let permanent_vars = permanent_variables(&rule);
    Ok(CompiledRule {
        key,
        head: rule.head,
        body: rule.body,
        permanent_vars,
    })
}

pub fn compile_facts(terms: Vec<Term>) -> WamResult<Vec<CompiledFact>> {
    terms.into_iter().map(compile_fact).collect()
}

fn unify_terms(left: &Term, right: &Term, bindings: &mut HashMap<String, Term>) -> bool {
    match (resolve(left, bindings), resolve(right, bindings)) {
        (Term::Variable(name), term) | (term, Term::Variable(name)) => bind(name, term, bindings),
        (Term::Atom(a), Term::Atom(b)) => a == b,
        (Term::String(a), Term::String(b)) => a == b,
        (Term::Number(a), Term::Number(b)) => (a - b).abs() < 1e-6,
        (Term::Compound { name: a, args: x }, Term::Compound { name: b, args: y }) => {
            a == b && unify_args(&x, &y, bindings)
        }
        _ => false,
    }
}

fn unify_args(left: &[Term], right: &[Term], bindings: &mut HashMap<String, Term>) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(a, b)| unify_terms(a, b, bindings))
}

fn bind(name: String, term: Term, bindings: &mut HashMap<String, Term>) -> bool {
    if name == "_" {
        return true;
    }
    bindings.insert(name, term);
    true
}

fn resolve(term: &Term, bindings: &HashMap<String, Term>) -> Term {
    match term {
        Term::Variable(name) => bindings
            .get(name)
            .map(|value| resolve(value, bindings))
            .unwrap_or_else(|| term.clone()),
        _ => term.clone(),
    }
}

fn substitute(term: &Term, bindings: &HashMap<String, Term>) -> Term {
    match term {
        Term::Variable(name) => bindings.get(name).cloned().unwrap_or_else(|| term.clone()),
        Term::Compound { name, args } => Term::Compound {
            name: name.clone(),
            args: args.iter().map(|arg| substitute(arg, bindings)).collect(),
        },
        _ => term.clone(),
    }
}

fn normalize(bindings: HashMap<String, Term>) -> HashMap<String, Term> {
    bindings
        .keys()
        .map(|name| {
            (
                name.clone(),
                resolve(&Term::Variable(name.clone()), &bindings),
            )
        })
        .collect()
}

fn merge(left: &HashMap<String, Term>, right: HashMap<String, Term>) -> HashMap<String, Term> {
    let mut merged = left.clone();
    merged.extend(right);
    merged
}
