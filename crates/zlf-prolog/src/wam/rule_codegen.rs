use crate::parser::{PrologRule, Term};

use super::codegen::WamCodegen;
use super::error::{WamError, WamResult};
use super::instruction::Instruction;
use super::predicate::{compound_args, predicate_key};
use super::program::WamProgram;

impl WamCodegen {
    pub fn compile_rule_clause(rule: &PrologRule) -> WamResult<WamProgram> {
        let args = compound_args(&rule.head).ok_or(WamError::ExpectedFunctor(0))?;
        Self::compile_rule_clause_with_temp_start(rule, args.len())
    }

    pub(crate) fn compile_rule_clause_with_temp_start(
        rule: &PrologRule,
        temp_start: usize,
    ) -> WamResult<WamProgram> {
        let args = compound_args(&rule.head).ok_or(WamError::ExpectedFunctor(0))?;
        let mut codegen = Self::with_temp_start(temp_start.max(args.len()));
        let mut instructions = vec![Instruction::Allocate];
        for (index, arg) in args.iter().enumerate() {
            codegen.compile_get(arg, index, &mut instructions)?;
        }
        for goal in &rule.body {
            codegen.compile_body_goal(goal, &mut instructions)?;
        }
        instructions.push(Instruction::Deallocate);
        instructions.push(Instruction::Proceed);
        Ok(WamProgram::new(instructions))
    }

    pub(crate) fn compile_body_goal(
        &mut self,
        goal: &Term,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        if matches!(goal, Term::Atom(name) if name == "!") {
            instructions.push(Instruction::Cut);
            return Ok(());
        }
        let key = predicate_key(goal).ok_or(WamError::ExpectedFunctor(0))?;
        let args = compound_args(goal).ok_or(WamError::ExpectedFunctor(0))?;
        for (index, arg) in args.iter().enumerate() {
            self.compile_put(arg, index, instructions)?;
        }
        instructions.push(Instruction::Call(key));
        Ok(())
    }
}
