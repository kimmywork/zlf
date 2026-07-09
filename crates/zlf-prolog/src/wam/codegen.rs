use std::collections::HashMap;

use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::instruction::Instruction;
use super::predicate::{compound_args, predicate_key, PredicateKey};
use super::program::WamProgram;

#[derive(Debug, Default)]
pub struct WamCodegen {
    pub(crate) var_regs: HashMap<String, usize>,
    pub(crate) permanent_slots: HashMap<String, usize>,
    pub(crate) next_temp: usize,
}

impl WamCodegen {
    pub fn compile_query_goal(goal: &Term) -> WamResult<WamProgram> {
        let key = predicate_key(goal).ok_or(WamError::ExpectedFunctor(0))?;
        let args = compound_args(goal).ok_or(WamError::ExpectedFunctor(0))?;
        let mut codegen = Self::with_temp_start(args.len());
        let mut instructions = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            codegen.compile_put(arg, index, &mut instructions)?;
        }
        instructions.push(Instruction::Call(key));
        Ok(WamProgram::new(instructions))
    }

    pub fn compile_fact_head(fact: &Term) -> WamResult<WamProgram> {
        let args = compound_args(fact).ok_or(WamError::ExpectedFunctor(0))?;
        Self::compile_fact_head_with_temp_start(fact, args.len())
    }

    pub(crate) fn compile_fact_head_with_temp_start(
        fact: &Term,
        temp_start: usize,
    ) -> WamResult<WamProgram> {
        let args = compound_args(fact).ok_or(WamError::ExpectedFunctor(0))?;
        let mut codegen = Self::with_temp_start(temp_start.max(args.len()));
        let mut instructions = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            codegen.compile_get(arg, index, &mut instructions)?;
        }
        instructions.push(Instruction::Proceed);
        Ok(WamProgram::new(instructions))
    }

    pub fn predicate_key(term: &Term) -> WamResult<PredicateKey> {
        predicate_key(term).ok_or(WamError::ExpectedFunctor(0))
    }

    pub(crate) fn with_temp_start(next_temp: usize) -> Self {
        Self {
            var_regs: HashMap::new(),
            permanent_slots: HashMap::new(),
            next_temp,
        }
    }

    pub(crate) fn with_permanent_slots(
        next_temp: usize,
        permanent_slots: HashMap<String, usize>,
    ) -> Self {
        Self {
            var_regs: HashMap::new(),
            permanent_slots,
            next_temp,
        }
    }

    pub(crate) fn compile_put(
        &mut self,
        term: &Term,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        match term {
            Term::Variable(name) => self.put_variable(name, register, instructions),
            Term::Compound { name, args } => self.put_structure(name, args, register, instructions),
            _ => self.put_constant(term, register, instructions),
        }
    }

    pub(crate) fn compile_get(
        &mut self,
        term: &Term,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        match term {
            Term::Variable(name) => self.get_variable(name, register, instructions),
            Term::Compound { name, args } => self.get_structure(name, args, register, instructions),
            _ => self.get_constant(term, register, instructions),
        }
    }
}
