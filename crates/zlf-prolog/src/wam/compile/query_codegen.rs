use std::collections::HashMap;

use crate::parser::Term;

use super::codegen::WamCodegen;
use super::error::{WamError, WamResult};
use super::instruction::Instruction;
use super::predicate::{compound_args, predicate_key};
use super::program::WamProgram;

#[derive(Debug, Clone)]
pub struct CompiledQuery {
    pub program: WamProgram,
    pub bindings: HashMap<String, usize>,
}

impl WamCodegen {
    pub fn compile_query_goal_with_bindings(goal: &Term) -> WamResult<CompiledQuery> {
        let args = compound_args(goal).ok_or(WamError::ExpectedFunctor(0))?;
        Self::compile_query_goal_with_binding_start(goal, args.len())
    }

    pub(crate) fn compile_query_goal_with_binding_start(
        goal: &Term,
        binding_start: usize,
    ) -> WamResult<CompiledQuery> {
        let key = predicate_key(goal).ok_or(WamError::ExpectedFunctor(0))?;
        let args = compound_args(goal).ok_or(WamError::ExpectedFunctor(0))?;
        let mut codegen = Self::with_temp_start(binding_start.max(args.len()));
        let mut instructions = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            codegen.compile_put(arg, index, &mut instructions)?;
        }
        let bindings = codegen.preserve_query_bindings(&mut instructions);
        instructions.push(Instruction::Call(key));
        Ok(CompiledQuery {
            program: WamProgram::new(instructions),
            bindings,
        })
    }

    fn preserve_query_bindings(
        &mut self,
        instructions: &mut Vec<Instruction>,
    ) -> HashMap<String, usize> {
        let mut bindings = HashMap::new();
        for (name, source) in self.var_regs.clone() {
            let target = self.next_temp;
            self.next_temp += 1;
            instructions.push(Instruction::PutValue { source, target });
            bindings.insert(name, target);
        }
        bindings
    }
}
