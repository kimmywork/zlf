use crate::parser::Term;

use super::choice_control;
use super::choice_point::ChoicePointFrame;
use super::environment_stack::EnvironmentStack;
use super::error::WamResult;
use super::execution_result::{ExecutionResult, StepOutcome};
use super::instruction::Instruction;
use super::machine::M0Machine;
use super::predicate::PredicateKey;
use super::program::WamProgram;
use super::register::RegisterFile;
use super::structure_mode::StructureMode;
use super::structure_ops;
use super::term_reader::term_from_heap;

#[derive(Debug)]
pub struct WamExecutor {
    pub(crate) machine: M0Machine,
    pub(crate) registers: RegisterFile,
    pub(crate) last_call: Option<PredicateKey>,
    pub(crate) mode: StructureMode,
    pub(crate) environments: EnvironmentStack,
    pub(crate) choice_points: Vec<ChoicePointFrame>,
    pub(crate) call_stack: Vec<usize>,
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
        }
    }

    pub fn execute(&mut self, program: &WamProgram) -> WamResult<ExecutionResult> {
        self.reset_run_state();
        let mut pc = 0;
        while let Some(instruction) = program.instructions().get(pc) {
            if matches!(instruction, Instruction::Proceed) {
                if let Some(target) = self.return_from_call() {
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
    }

    pub(crate) fn step_or_jump(
        &mut self,
        instruction: &Instruction,
        program: &WamProgram,
        return_pc: usize,
    ) -> WamResult<StepOutcome> {
        match instruction {
            Instruction::Call(key) => {
                self.call(key)?;
                return Ok(program.entry(key).map_or(StepOutcome::Continue, |target| {
                    self.call_stack.push(return_pc);
                    StepOutcome::Jump(target)
                }));
            }
            Instruction::Execute(key) => {
                self.call(key)?;
                return Ok(program
                    .entry(key)
                    .map_or(StepOutcome::Continue, StepOutcome::Jump));
            }
            _ => {}
        }
        if self.step(instruction)? {
            Ok(StepOutcome::Continue)
        } else {
            Ok(StepOutcome::Failed)
        }
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
            Instruction::PutConstant { value, register } => self.put_constant(value, *register),
            Instruction::PutStructure {
                name,
                arity,
                register,
            } => self.put_structure(name, *arity, *register),
            Instruction::SetVariable { register } => self.set_variable(*register),
            Instruction::SetValue { register } => self.set_value(*register),
            Instruction::SetConstant { value } => self.set_constant(value),
            Instruction::GetConstant { value, register } => self.get_constant(value, *register),
            Instruction::GetStructure {
                name,
                arity,
                register,
            } => self.get_structure(name, *arity, *register),
            Instruction::GetValue { left, right } => self.unify_registers(*left, *right),
            Instruction::UnifyConstant { value } => self.unify_constant(value),
            Instruction::UnifyVariable { register } => self.unify_variable(*register),
            Instruction::UnifyValue { register } => self.unify_value(*register),
            Instruction::UnifyRegisters { left, right } => self.unify_registers(*left, *right),
            Instruction::Call(key) | Instruction::Execute(key) => self.call(key),
            Instruction::Allocate => self.allocate(),
            Instruction::Deallocate => self.deallocate(),
            Instruction::TryMeElse(next) => self.try_me_else(*next),
            Instruction::RetryMeElse(next) => self.retry_me_else(*next),
            Instruction::TrustMe => self.trust_me(),
            _ => Ok(true),
        }
    }

    pub(crate) fn result(&self, success: bool) -> ExecutionResult {
        ExecutionResult {
            success,
            last_call: self.last_call.clone(),
        }
    }

    pub(crate) fn backtrack_target(&self) -> Option<usize> {
        self.choice_points
            .last()
            .map(ChoicePointFrame::next_alternative)
    }

    pub(crate) fn return_from_call(&mut self) -> Option<usize> {
        self.call_stack.pop()
    }

    fn put_variable(&mut self, register: usize) -> WamResult<bool> {
        let addr = self.machine.set_variable();
        self.registers.set(register, addr)?;
        Ok(true)
    }

    fn put_value(&mut self, source: usize, target: usize) -> WamResult<bool> {
        let addr = self.registers.get(source)?;
        self.registers.set(target, addr)?;
        Ok(true)
    }

    fn put_constant(&mut self, value: &str, register: usize) -> WamResult<bool> {
        let addr = self.machine.put_constant(value);
        self.registers.set(register, addr)?;
        Ok(true)
    }

    fn put_structure(&mut self, name: &str, arity: usize, register: usize) -> WamResult<bool> {
        structure_ops::put_structure(
            &mut self.machine,
            &mut self.registers,
            &mut self.mode,
            name,
            arity,
            register,
        )
    }

    fn set_variable(&mut self, register: usize) -> WamResult<bool> {
        let addr = self.machine.set_variable();
        self.registers.set(register, addr)?;
        self.mode.maybe_consume_write_arg()?;
        Ok(true)
    }

    fn set_value(&mut self, register: usize) -> WamResult<bool> {
        let addr = self.registers.get(register)?;
        self.machine.set_value(addr)?;
        self.mode.maybe_consume_write_arg()?;
        Ok(true)
    }

    fn set_constant(&mut self, value: &str) -> WamResult<bool> {
        self.machine.put_constant(value);
        self.mode.maybe_consume_write_arg()?;
        Ok(true)
    }

    fn get_constant(&mut self, value: &str, register: usize) -> WamResult<bool> {
        let constant = self.machine.put_constant(value);
        let addr = self.registers.get(register)?;
        self.machine.unify(addr, constant)
    }

    fn get_structure(&mut self, name: &str, arity: usize, register: usize) -> WamResult<bool> {
        structure_ops::get_structure(
            &mut self.machine,
            &mut self.registers,
            &mut self.mode,
            name,
            arity,
            register,
        )
    }

    fn unify_constant(&mut self, value: &str) -> WamResult<bool> {
        structure_ops::unify_constant(&mut self.machine, &mut self.mode, value)
    }

    fn unify_variable(&mut self, register: usize) -> WamResult<bool> {
        structure_ops::unify_variable(
            &mut self.machine,
            &mut self.registers,
            &mut self.mode,
            register,
        )
    }

    fn unify_value(&mut self, register: usize) -> WamResult<bool> {
        structure_ops::unify_value(
            &mut self.machine,
            &mut self.registers,
            &mut self.mode,
            register,
        )
    }

    fn unify_registers(&mut self, left: usize, right: usize) -> WamResult<bool> {
        let left_addr = self.registers.get(left)?;
        let right_addr = self.registers.get(right)?;
        self.machine.unify(left_addr, right_addr)
    }

    fn call(&mut self, key: &PredicateKey) -> WamResult<bool> {
        self.last_call = Some(key.clone());
        Ok(true)
    }

    fn allocate(&mut self) -> WamResult<bool> {
        self.environments.allocate(None);
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
            &self.call_stack,
            next,
        );
        Ok(true)
    }

    fn retry_me_else(&mut self, next: usize) -> WamResult<bool> {
        choice_control::retry_me_else(
            &mut self.choice_points,
            &mut self.machine,
            &mut self.registers,
            &mut self.call_stack,
            next,
        )?;
        Ok(true)
    }

    fn trust_me(&mut self) -> WamResult<bool> {
        choice_control::trust_me(
            &mut self.choice_points,
            &mut self.machine,
            &mut self.registers,
            &mut self.call_stack,
        )?;
        Ok(true)
    }
}
