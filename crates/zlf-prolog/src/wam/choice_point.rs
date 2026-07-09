use super::environment_stack::EnvironmentStack;
use super::error::WamResult;
use super::machine::M0Machine;
use super::register::RegisterFile;

#[derive(Debug, Clone)]
pub struct ChoicePointFrame {
    heap_checkpoint: usize,
    trail_checkpoint: usize,
    registers: Vec<Option<usize>>,
    environments: EnvironmentStack,
    call_stack: Vec<usize>,
    cut_base_stack: Vec<usize>,
    continuation: Option<usize>,
    next_alternative: usize,
}

impl ChoicePointFrame {
    pub fn capture(
        machine: &M0Machine,
        registers: &RegisterFile,
        environments: &EnvironmentStack,
        continuation: Option<usize>,
        next_alternative: usize,
    ) -> Self {
        Self {
            heap_checkpoint: machine.heap_checkpoint(),
            trail_checkpoint: machine.trail_checkpoint(),
            registers: registers.snapshot(),
            environments: environments.clone(),
            call_stack: Vec::new(),
            cut_base_stack: Vec::new(),
            continuation,
            next_alternative,
        }
    }

    pub fn capture_with_call_stack(
        machine: &M0Machine,
        registers: &RegisterFile,
        environments: &EnvironmentStack,
        call_stack: &[usize],
        cut_base_stack: &[usize],
        continuation: Option<usize>,
        next_alternative: usize,
    ) -> Self {
        Self {
            heap_checkpoint: machine.heap_checkpoint(),
            trail_checkpoint: machine.trail_checkpoint(),
            registers: registers.snapshot(),
            environments: environments.clone(),
            call_stack: call_stack.to_vec(),
            cut_base_stack: cut_base_stack.to_vec(),
            continuation,
            next_alternative,
        }
    }

    pub fn restore(
        &self,
        machine: &mut M0Machine,
        registers: &mut RegisterFile,
        environments: &mut EnvironmentStack,
    ) -> WamResult<()> {
        machine.unwind_trail(self.trail_checkpoint)?;
        machine.unwind_heap(self.heap_checkpoint)?;
        registers.restore(self.registers.clone());
        *environments = self.environments.clone();
        Ok(())
    }

    pub fn call_stack(&self) -> Vec<usize> {
        self.call_stack.clone()
    }

    pub fn cut_base_stack(&self) -> Vec<usize> {
        self.cut_base_stack.clone()
    }

    pub fn continuation(&self) -> Option<usize> {
        self.continuation
    }

    pub fn next_alternative(&self) -> usize {
        self.next_alternative
    }

    pub fn retarget(&mut self, next_alternative: usize) {
        self.next_alternative = next_alternative;
    }

    pub fn heap_checkpoint(&self) -> usize {
        self.heap_checkpoint
    }

    pub fn trail_checkpoint(&self) -> usize {
        self.trail_checkpoint
    }
}
