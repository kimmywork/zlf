use crate::parser::{PrologRule, Term};

use super::codegen::WamCodegen;
use super::error::{WamError, WamResult};
use super::instruction::Instruction;
use super::permanent_vars::permanent_slots;
use super::predicate::{compound_args, predicate_key};
use super::program::WamProgram;
use super::proof::ProofClause;

impl WamCodegen {
    pub fn compile_rule_clause(rule: &PrologRule) -> WamResult<WamProgram> {
        let args = compound_args(&rule.head).ok_or(WamError::ExpectedFunctor(0))?;
        Self::compile_rule_clause_with_temp_start(rule, args.len())
    }

    pub(crate) fn compile_rule_clause_with_temp_start(
        rule: &PrologRule,
        temp_start: usize,
    ) -> WamResult<WamProgram> {
        Self::compile_rule_clause_with_proof(rule, temp_start, None)
    }

    pub(crate) fn compile_rule_clause_with_proof(
        rule: &PrologRule,
        temp_start: usize,
        proof: Option<ProofClause>,
    ) -> WamResult<WamProgram> {
        let args = compound_args(&rule.head).ok_or(WamError::ExpectedFunctor(0))?;
        let slots = permanent_slots(rule);
        let mut codegen = Self::with_permanent_slots(temp_start.max(args.len()), slots.clone());
        let allocate = if slots.is_empty() {
            Instruction::Allocate
        } else {
            Instruction::AllocatePermanent {
                permanent_count: slots.len(),
            }
        };
        let mut instructions = vec![allocate];
        for (index, arg) in args.iter().enumerate() {
            codegen.compile_get(arg, index, &mut instructions)?;
        }
        if let Some(proof) = proof {
            instructions.push(Instruction::EnterProof(proof));
        }
        if let Some((last, leading)) = rule.body.split_last() {
            for goal in leading {
                codegen.compile_body_goal(goal, &mut instructions)?;
            }
            codegen.compile_tail_goal(last, &mut instructions)?;
        } else {
            instructions.push(Instruction::Deallocate);
            instructions.push(Instruction::Proceed);
        }
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
        let key = self.compile_goal_args(goal, instructions)?;
        instructions.push(Instruction::Call(key));
        Ok(())
    }

    fn compile_tail_goal(
        &mut self,
        goal: &Term,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        if matches!(goal, Term::Atom(name) if name == "!") {
            instructions.push(Instruction::Cut);
            instructions.push(Instruction::Deallocate);
            instructions.push(Instruction::Proceed);
            return Ok(());
        }
        let key = self.compile_goal_args(goal, instructions)?;
        instructions.push(Instruction::Deallocate);
        instructions.push(Instruction::Execute(key));
        Ok(())
    }

    fn compile_goal_args(
        &mut self,
        goal: &Term,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<super::predicate::PredicateKey> {
        let key = predicate_key(goal).ok_or(WamError::ExpectedFunctor(0))?;
        let args = compound_args(goal).ok_or(WamError::ExpectedFunctor(0))?;
        for (index, arg) in args.iter().enumerate() {
            self.compile_put(arg, index, instructions)?;
        }
        Ok(key)
    }
}
