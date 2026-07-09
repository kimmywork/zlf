use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::parser::Term;

use super::cell::Cell;
use super::compiler::M0Compiler;
use super::error::{WamError, WamResult};
use super::machine::M0Machine;
use super::register::RegisterFile;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PredicateKey {
    pub name: String,
    pub arity: usize,
}

#[derive(Debug, Clone)]
pub struct CompiledFact {
    pub key: PredicateKey,
    pub head: Term,
}

#[derive(Debug)]
pub struct M1Machine {
    machine: M0Machine,
    registers: RegisterFile,
}

impl M1Machine {
    pub fn new(register_count: usize) -> Self {
        Self {
            machine: M0Machine::new(),
            registers: RegisterFile::new(register_count),
        }
    }

    pub fn solve_fact(
        &mut self,
        goal: &Term,
        fact: &CompiledFact,
    ) -> WamResult<Option<HashMap<String, Term>>> {
        let goal_key = predicate_key(goal).ok_or(WamError::ExpectedFunctor(0))?;
        if goal_key != fact.key {
            return Ok(None);
        }
        self.registers.clear();
        let mut goal_compiler = self.load_goal_registers(goal)?;
        if !self.match_fact_args(&fact.head)? {
            return Ok(None);
        }
        self.collect_bindings(&mut goal_compiler).map(Some)
    }

    fn load_goal_registers(&mut self, goal: &Term) -> WamResult<M0Compiler> {
        let args = compound_args(goal).ok_or(WamError::ExpectedFunctor(0))?;
        let mut compiler = M0Compiler::new();
        for (index, arg) in args.iter().enumerate() {
            let addr = compiler.compile_term(&mut self.machine, arg)?;
            self.registers.set(index, addr)?;
        }
        Ok(compiler)
    }

    fn match_fact_args(&mut self, fact: &Term) -> WamResult<bool> {
        let args = compound_args(fact).ok_or(WamError::ExpectedFunctor(0))?;
        let mut compiler = M0Compiler::new();
        for (index, arg) in args.iter().enumerate() {
            let fact_addr = compiler.compile_term(&mut self.machine, arg)?;
            let reg_addr = self.registers.get(index)?;
            if !self.machine.unify(reg_addr, fact_addr)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn collect_bindings(&self, compiler: &mut M0Compiler) -> WamResult<HashMap<String, Term>> {
        compiler
            .variables()
            .iter()
            .map(|(name, addr)| Ok((name.clone(), self.term_from_heap(*addr)?)))
            .collect()
    }

    fn term_from_heap(&self, addr: usize) -> WamResult<Term> {
        let deref = self.machine.heap().deref(addr)?;
        match self.machine.heap().cell(deref)? {
            Cell::Constant(value) => Ok(Term::Atom(value.clone())),
            Cell::Str(_) => self.structure_from_heap(deref),
            Cell::Ref(_) => Ok(Term::Variable("_".to_string())),
            Cell::Functor { .. } => Err(WamError::ExpectedFunctor(deref)),
        }
    }

    fn structure_from_heap(&self, addr: usize) -> WamResult<Term> {
        let (name, arity, first_arg) = self.machine.heap().structure_parts(addr)?;
        let args = (0..arity)
            .map(|offset| self.term_from_heap(first_arg + offset))
            .collect::<WamResult<Vec<_>>>()?;
        Ok(Term::Compound {
            name: name.to_string(),
            args,
        })
    }
}

pub fn compile_fact(term: Term) -> WamResult<CompiledFact> {
    let key = predicate_key(&term).ok_or(WamError::ExpectedFunctor(0))?;
    Ok(CompiledFact { key, head: term })
}

pub fn predicate_key(term: &Term) -> Option<PredicateKey> {
    match term {
        Term::Compound { name, args } => Some(PredicateKey {
            name: name.clone(),
            arity: args.len(),
        }),
        Term::Atom(name) => Some(PredicateKey {
            name: name.clone(),
            arity: 0,
        }),
        _ => None,
    }
}

pub(crate) fn compound_args(term: &Term) -> Option<&[Term]> {
    match term {
        Term::Compound { args, .. } => Some(args),
        Term::Atom(_) => Some(&[]),
        _ => None,
    }
}
