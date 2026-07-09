use crate::parser::Term;

use super::error::WamResult;
use super::execution_result::StepOutcome;
use super::instruction::Instruction;
use super::program::WamProgram;
use super::WamExecutor;

impl WamExecutor {
    pub fn execute_all_registers(
        &mut self,
        program: &WamProgram,
        registers: &[usize],
    ) -> WamResult<Vec<Vec<Term>>> {
        self.reset_run_state();
        let mut pc = 0;
        let mut solutions = Vec::new();
        while let Some(instruction) = program.instructions().get(pc) {
            if matches!(instruction, Instruction::Proceed) {
                if let Some(target) = self.return_from_call() {
                    pc = target;
                    continue;
                }
                solutions.push(self.collect_registers(registers)?);
                if let Some(target) = self.backtrack_target() {
                    pc = target;
                    continue;
                }
                break;
            }
            match self.step_or_jump(instruction, program, pc + 1)? {
                StepOutcome::Continue => pc += 1,
                StepOutcome::Jump(target) => pc = target,
                StepOutcome::Failed => match self.backtrack_target() {
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
