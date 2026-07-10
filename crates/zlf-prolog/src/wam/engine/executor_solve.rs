use crate::parser::Term;

use super::error::WamResult;
use super::execution_result::StepOutcome;
use super::fact_provider::FactProvider;
use super::instruction::Instruction;
use super::program::WamProgram;
use super::proof::{ProofState, ProofTree};
use super::WamExecutor;
use zlf_storage::Storage;

impl WamExecutor {
    pub fn execute_all_registers(
        &mut self,
        program: &WamProgram,
        registers: &[usize],
    ) -> WamResult<Vec<Vec<Term>>> {
        self.execute_all_registers_with_context(program, registers, None, None)
    }

    pub(crate) fn execute_all_registers_with_context(
        &mut self,
        program: &WamProgram,
        registers: &[usize],
        provider: Option<&dyn FactProvider>,
        storage: Option<&Storage>,
    ) -> WamResult<Vec<Vec<Term>>> {
        Ok(self
            .execute_register_rows(program, registers, provider, storage)?
            .into_iter()
            .map(|(row, _)| row)
            .collect())
    }

    pub(crate) fn execute_all_registers_with_proof(
        &mut self,
        program: &WamProgram,
        registers: &[usize],
        provider: Option<&dyn FactProvider>,
        storage: Option<&Storage>,
    ) -> WamResult<Vec<(Vec<Term>, ProofTree)>> {
        self.proof = ProofState::enabled();
        self.execute_register_rows(program, registers, provider, storage)
    }

    fn execute_register_rows(
        &mut self,
        program: &WamProgram,
        registers: &[usize],
        provider: Option<&dyn FactProvider>,
        storage: Option<&Storage>,
    ) -> WamResult<Vec<(Vec<Term>, ProofTree)>> {
        self.reset_run_state();
        let mut pc = 0;
        let mut solutions = Vec::new();
        while let Some(instruction) = program.instructions().get(pc) {
            if matches!(instruction, Instruction::Proceed) {
                self.proof.complete_depth(self.call_stack.len());
                if let Some(target) = self.return_from_call() {
                    pc = target;
                    continue;
                }
                solutions.push((self.collect_registers(registers)?, self.proof.snapshot()));
                if let Some(target) = self.backtrack_target()? {
                    pc = target;
                    continue;
                }
                break;
            }
            match self.step_or_jump_with_context(instruction, program, pc + 1, provider, storage)? {
                StepOutcome::Continue => pc += 1,
                StepOutcome::Jump(target) => pc = target,
                StepOutcome::Failed => match self.backtrack_target()? {
                    Some(target) => pc = target,
                    None => break,
                },
            }
        }
        Ok(solutions)
    }

    fn collect_registers(&self, registers: &[usize]) -> WamResult<Vec<Term>> {
        registers
            .iter()
            .map(|register| self.register_term(*register))
            .collect()
    }
}
