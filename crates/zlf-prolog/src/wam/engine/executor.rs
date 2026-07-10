use crate::parser::Term;

use super::choice_control;
use super::choice_point::ChoicePointFrame;
use super::environment_stack::EnvironmentStack;
use super::error::WamResult;
use super::execution_result::{ExecutionResult, StepOutcome};
use super::fact_provider::FactProvider;
use super::instruction::Instruction;
use super::machine::M0Machine;
use super::predicate::PredicateKey;
use super::program::WamProgram;
use super::proof::ProofState;
use super::register::RegisterFile;
use super::structure_mode::StructureMode;
use super::term_reader::term_from_heap;
use zlf_storage::Storage;

#[derive(Debug)]
pub struct WamExecutor {
    pub(crate) machine: M0Machine,
    pub(crate) registers: RegisterFile,
    pub(crate) last_call: Option<PredicateKey>,
    pub(crate) mode: StructureMode,
    pub(crate) environments: EnvironmentStack,
    pub(crate) choice_points: Vec<ChoicePointFrame>,
    pub(crate) call_stack: Vec<usize>,
    pub(crate) cut_base_stack: Vec<usize>,
    pub(crate) proof: ProofState,
}

impl WamExecutor {
    pub fn new(register_count: usize) -> Self {
        Self {
            machine: M0Machine::new(),
            registers: RegisterFile::new(register_count),
            last_call: None,
            mode: StructureMode::None,
            environments: EnvironmentStack::new(),
            choice_points: Vec::new(),
            call_stack: Vec::new(),
            cut_base_stack: Vec::new(),
            proof: ProofState::default(),
        }
    }

    pub fn execute(&mut self, program: &WamProgram) -> WamResult<ExecutionResult> {
        self.reset_run_state();
        let mut pc = 0;
        while let Some(instruction) = program.instructions().get(pc) {
            if matches!(instruction, Instruction::Proceed) {
                self.proof.complete_depth(self.call_stack.len());
                if let Some(target) = self.return_from_call() {
                    pc = target;
                    continue;
                }
                break;
            }
            match self.step_or_jump(instruction, program, pc + 1)? {
                StepOutcome::Continue => pc += 1,
                StepOutcome::Jump(target) => pc = target,
                StepOutcome::Failed => match self.backtrack_target()? {
                    Some(target) => pc = target,
                    None => return Ok(self.result(false)),
                },
            }
        }
        Ok(self.result(true))
    }

    pub(crate) fn reset_run_state(&mut self) {
        self.last_call = None;
        self.mode = StructureMode::None;
        self.environments.clear();
        self.choice_points.clear();
        self.call_stack.clear();
        self.cut_base_stack.clear();
        self.proof.reset();
    }

    pub(crate) fn step_or_jump(
        &mut self,
        instruction: &Instruction,
        program: &WamProgram,
        return_pc: usize,
    ) -> WamResult<StepOutcome> {
        self.step_or_jump_with_context(instruction, program, return_pc, None, None)
    }

    pub(crate) fn step_or_jump_with_context(
        &mut self,
        instruction: &Instruction,
        program: &WamProgram,
        return_pc: usize,
        provider: Option<&dyn FactProvider>,
        storage: Option<&Storage>,
    ) -> WamResult<StepOutcome> {
        if let Some(outcome) = self.switch_step(instruction)? {
            return Ok(outcome);
        }
        match instruction {
            Instruction::Call(key) => {
                return self.call_outcome(key, program, Some(return_pc), provider, storage)
            }
            Instruction::Execute(key) => {
                return self.call_outcome(key, program, None, provider, storage)
            }
            _ => {}
        }
        if self.step(instruction)? {
            Ok(StepOutcome::Continue)
        } else {
            Ok(StepOutcome::Failed)
        }
    }

    fn call_outcome(
        &mut self,
        key: &PredicateKey,
        program: &WamProgram,
        return_pc: Option<usize>,
        provider: Option<&dyn FactProvider>,
        storage: Option<&Storage>,
    ) -> WamResult<StepOutcome> {
        self.call(key)?;
        if key.name == "call" && key.arity > 0 {
            return self.meta_call_outcome(key.arity, program, return_pc, provider, storage);
        }
        self.dispatch_call(key, program, return_pc, provider, storage)
    }

    pub fn register_term(&self, register: usize) -> WamResult<Term> {
        term_from_heap(self.machine.heap(), self.registers.get(register)?)
    }

    pub fn environment_depth(&self) -> usize {
        self.environments.depth()
    }

    pub fn choice_point_depth(&self) -> usize {
        self.choice_points.len()
    }

    pub fn next_alternative(&self) -> Option<usize> {
        self.choice_points
            .last()
            .map(ChoicePointFrame::next_alternative)
    }

    #[allow(clippy::too_many_lines)]
    fn step(&mut self, instruction: &Instruction) -> WamResult<bool> {
        match instruction {
            Instruction::PutVariable { register } => self.put_variable(*register),
            Instruction::PutValue { source, target } => self.put_value(*source, *target),
            Instruction::PutPermanentValue { slot, register } => {
                self.put_permanent_value(*slot, *register)
            }
            Instruction::PutConstant { value, register } => self.put_constant(value, *register),
            Instruction::PutStructure {
                name,
                arity,
                register,
            } => self.put_structure(name, *arity, *register),
            Instruction::SetVariable { register } => self.set_variable(*register),
            Instruction::SetValue { register } => self.set_value(*register),
            Instruction::SetPermanentValue { slot } => self.set_permanent_value(*slot),
            Instruction::SetConstant { value } => self.set_constant(value),
            Instruction::GetConstant { value, register } => self.get_constant(value, *register),
            Instruction::GetStructure {
                name,
                arity,
                register,
            } => self.get_structure(name, *arity, *register),
            Instruction::GetValue { left, right } => self.unify_registers(*left, *right),
            Instruction::GetPermanentValue { slot, register } => {
                self.get_permanent_value(*slot, *register)
            }
            Instruction::UnifyConstant { value } => self.unify_constant(value),
            Instruction::UnifyVariable { register } => self.unify_variable(*register),
            Instruction::UnifyValue { register } => self.unify_value(*register),
            Instruction::UnifyPermanentValue { slot } => self.unify_permanent_value(*slot),
            Instruction::UnifyRegisters { left, right } => self.unify_registers(*left, *right),
            Instruction::EnterProof(clause) => self.enter_proof(clause),
            Instruction::Call(key) | Instruction::Execute(key) => self.call(key),
            Instruction::Allocate => self.allocate(0),
            Instruction::AllocatePermanent { permanent_count } => self.allocate(*permanent_count),
            Instruction::Deallocate => self.deallocate(),
            Instruction::TryMeElse(next) => self.try_me_else(*next),
            Instruction::RetryMeElse(next) => self.retry_me_else(*next),
            Instruction::TrustMe => self.trust_me(),
            Instruction::Cut | Instruction::NeckCut => self.cut(),
            Instruction::GetLevel { slot } => self.get_level(*slot),
            Instruction::CutLevel { slot } => self.cut_level(*slot),
            _ => Ok(true),
        }
    }

    pub(crate) fn result(&self, success: bool) -> ExecutionResult {
        ExecutionResult {
            success,
            last_call: self.last_call.clone(),
        }
    }

    pub(crate) fn return_from_call(&mut self) -> Option<usize> {
        let target = self.call_stack.pop();
        if target.is_some() {
            self.cut_base_stack.pop();
        }
        target
    }

    fn call(&mut self, key: &PredicateKey) -> WamResult<bool> {
        self.last_call = Some(key.clone());
        Ok(true)
    }

    fn allocate(&mut self, permanent_count: usize) -> WamResult<bool> {
        let cut_base = self
            .cut_base_stack
            .last()
            .copied()
            .unwrap_or(self.choice_points.len());
        self.environments.allocate(None, cut_base, permanent_count);
        Ok(true)
    }

    fn deallocate(&mut self) -> WamResult<bool> {
        self.environments.deallocate()?;
        Ok(true)
    }

    fn try_me_else(&mut self, next: usize) -> WamResult<bool> {
        choice_control::try_me_else(
            &mut self.choice_points,
            &self.machine,
            &self.registers,
            &self.environments,
            &self.call_stack,
            &self.cut_base_stack,
            next,
            self.proof.checkpoint(),
        );
        Ok(true)
    }

    fn retry_me_else(&mut self, next: usize) -> WamResult<bool> {
        let proof_checkpoint = choice_control::retry_me_else(
            &mut self.choice_points,
            &mut self.machine,
            &mut self.registers,
            &mut self.environments,
            &mut self.call_stack,
            &mut self.cut_base_stack,
            next,
        )?;
        self.proof.restore(proof_checkpoint);
        Ok(true)
    }

    fn trust_me(&mut self) -> WamResult<bool> {
        let proof_checkpoint = choice_control::trust_me(
            &mut self.choice_points,
            &mut self.machine,
            &mut self.registers,
            &mut self.environments,
            &mut self.call_stack,
            &mut self.cut_base_stack,
        )?;
        self.proof.restore(proof_checkpoint);
        Ok(true)
    }

    fn cut(&mut self) -> WamResult<bool> {
        let base = self.environments.cut_base().unwrap_or_default();
        self.truncate_choice_points(base);
        Ok(true)
    }

    fn get_level(&mut self, slot: usize) -> WamResult<bool> {
        let level = self.environments.cut_base().unwrap_or_default();
        self.environments.set_cut_level(slot, level)?;
        Ok(true)
    }

    fn cut_level(&mut self, slot: usize) -> WamResult<bool> {
        let level = self.environments.cut_level(slot)?.unwrap_or_default();
        self.truncate_choice_points(level);
        Ok(true)
    }

    fn truncate_choice_points(&mut self, level: usize) {
        self.choice_points
            .truncate(level.min(self.choice_points.len()));
    }
}
