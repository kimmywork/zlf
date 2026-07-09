use std::collections::HashMap;

use crate::parser::Term;

use super::error::WamResult;
use super::m1::{compile_fact, predicate_key, CompiledFact, M1Machine, PredicateKey};
use super::m2::{compile_rule, CompiledRule};

pub type Bindings = HashMap<String, Term>;

#[derive(Debug, Clone)]
pub struct ChoicePoint {
    alternatives: Vec<Bindings>,
    next: usize,
}

#[derive(Debug, Clone, Default)]
pub struct M3Program {
    facts: Vec<CompiledFact>,
    rules: Vec<CompiledRule>,
}

#[derive(Debug)]
pub struct M3Machine {
    register_count: usize,
}

impl ChoicePoint {
    pub fn new(alternatives: Vec<Bindings>) -> Self {
        Self {
            alternatives,
            next: 0,
        }
    }

    pub fn next_solution(&mut self) -> Option<Bindings> {
        let solution = self.alternatives.get(self.next).cloned();
        self.next += usize::from(solution.is_some());
        solution
    }
}

impl M3Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_fact(&mut self, fact: Term) -> WamResult<()> {
        self.facts.push(compile_fact(fact)?);
        Ok(())
    }

    pub fn add_rule(&mut self, rule: crate::parser::PrologRule) -> WamResult<()> {
        self.rules.push(compile_rule(rule)?);
        Ok(())
    }
}

impl M3Machine {
    pub fn new(register_count: usize) -> Self {
        Self { register_count }
    }

    pub fn solve(&self, goal: &Term, program: &M3Program) -> WamResult<Vec<Bindings>> {
        let solutions = self.solve_goals(&[goal.clone()], Bindings::new(), program)?;
        Ok(solutions.into_iter().map(normalize).collect())
    }

    fn solve_goals(
        &self,
        goals: &[Term],
        bindings: Bindings,
        program: &M3Program,
    ) -> WamResult<Vec<Bindings>> {
        let Some((first, rest)) = goals.split_first() else {
            return Ok(vec![bindings]);
        };
        let goal = substitute(first, &bindings);
        self.expand_choice_point(&goal, rest, bindings, program)
    }

    fn expand_choice_point(
        &self,
        goal: &Term,
        rest: &[Term],
        bindings: Bindings,
        program: &M3Program,
    ) -> WamResult<Vec<Bindings>> {
        let mut choice = ChoicePoint::new(self.alternatives(goal, program)?);
        let mut results = Vec::new();
        while let Some(solution) = choice.next_solution() {
            results.extend(self.solve_goals(rest, merge(&bindings, solution), program)?);
        }
        Ok(results)
    }

    fn alternatives(&self, goal: &Term, program: &M3Program) -> WamResult<Vec<Bindings>> {
        let mut results = self.fact_alternatives(goal, &program.facts)?;
        results.extend(self.rule_alternatives(goal, &program.rules, program)?);
        Ok(results)
    }

    fn fact_alternatives(&self, goal: &Term, facts: &[CompiledFact]) -> WamResult<Vec<Bindings>> {
        let mut results = Vec::new();
        for fact in facts.iter().filter(|fact| key_matches(goal, &fact.key)) {
            let mut machine = M1Machine::new(self.register_count);
            if let Some(solution) = machine.solve_fact(goal, fact)? {
                results.push(solution);
            }
        }
        Ok(results)
    }

    fn rule_alternatives(
        &self,
        goal: &Term,
        rules: &[CompiledRule],
        program: &M3Program,
    ) -> WamResult<Vec<Bindings>> {
        let mut results = Vec::new();
        for rule in rules.iter().filter(|rule| key_matches(goal, &rule.key)) {
            results.extend(self.solve_rule(goal, rule, program)?);
        }
        Ok(results)
    }

    fn solve_rule(
        &self,
        goal: &Term,
        rule: &CompiledRule,
        program: &M3Program,
    ) -> WamResult<Vec<Bindings>> {
        let mut bindings = Bindings::new();
        if !unify_terms(goal, &rule.head, &mut bindings) {
            return Ok(Vec::new());
        }
        self.solve_goals(&rule.body, bindings, program)
    }
}

fn key_matches(goal: &Term, key: &PredicateKey) -> bool {
    predicate_key(goal).as_ref() == Some(key)
}

fn unify_terms(left: &Term, right: &Term, bindings: &mut Bindings) -> bool {
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

fn unify_args(left: &[Term], right: &[Term], bindings: &mut Bindings) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(a, b)| unify_terms(a, b, bindings))
}

fn bind(name: String, term: Term, bindings: &mut Bindings) -> bool {
    if name != "_" {
        bindings.insert(name, term);
    }
    true
}

fn resolve(term: &Term, bindings: &Bindings) -> Term {
    match term {
        Term::Variable(name) => bindings
            .get(name)
            .map(|value| resolve(value, bindings))
            .unwrap_or_else(|| term.clone()),
        _ => term.clone(),
    }
}

fn substitute(term: &Term, bindings: &Bindings) -> Term {
    match term {
        Term::Variable(name) => bindings.get(name).cloned().unwrap_or_else(|| term.clone()),
        Term::Compound { name, args } => Term::Compound {
            name: name.clone(),
            args: args.iter().map(|arg| substitute(arg, bindings)).collect(),
        },
        _ => term.clone(),
    }
}

fn merge(left: &Bindings, right: Bindings) -> Bindings {
    let mut merged = left.clone();
    merged.extend(right);
    merged
}

fn normalize(bindings: Bindings) -> Bindings {
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
