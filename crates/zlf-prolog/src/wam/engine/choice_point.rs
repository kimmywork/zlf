use super::environment_stack::EnvironmentStack;
use super::error::WamResult;
use super::machine::M0Machine;
use super::predicate::PredicateKey;
use super::register::RegisterFile;
use crate::parser::Term;

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
    proof_checkpoint: usize,
    external_answers: Option<Vec<Vec<Term>>>,
    external_index: usize,
    external_tail_call: bool,
    external_predicate: Option<PredicateKey>,
}

impl ChoicePointFrame {
    pub fn capture(
        machine: &M0Machine,
        registers: &RegisterFile,
        environments: &EnvironmentStack,
        continuation: Option<usize>,
        next_alternative: usize,
    ) -> Self {
        Self::capture_with_call_stack(
            machine,
            registers,
            environments,
            &[],
            &[],
            continuation,
            next_alternative,
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn capture_with_call_stack(
        machine: &M0Machine,
        registers: &RegisterFile,
        environments: &EnvironmentStack,
        call_stack: &[usize],
        cut_base_stack: &[usize],
        continuation: Option<usize>,
        next_alternative: usize,
        proof_checkpoint: usize,
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
            proof_checkpoint,
            external_answers: None,
            external_index: 0,
            external_tail_call: false,
            external_predicate: None,
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

    pub fn proof_checkpoint(&self) -> usize {
        self.proof_checkpoint
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn capture_external(
        machine: &M0Machine,
        registers: &RegisterFile,
        environments: &EnvironmentStack,
        call_stack: &[usize],
        cut_base_stack: &[usize],
        continuation: usize,
        predicate: PredicateKey,
        answers: Vec<Vec<Term>>,
        proof_checkpoint: usize,
        tail_call: bool,
    ) -> Self {
        let mut frame = Self::capture_with_call_stack(
            machine,
            registers,
            environments,
            call_stack,
            cut_base_stack,
            Some(continuation),
            usize::MAX,
            proof_checkpoint,
        );
        frame.external_answers = Some(answers);
        frame.external_predicate = Some(predicate);
        frame.external_tail_call = tail_call;
        frame
    }

    pub(crate) fn is_external(&self) -> bool {
        self.external_answers.is_some()
    }

    pub(crate) fn next_external_answer(&mut self) -> Option<Vec<Term>> {
        let answer = self
            .external_answers
            .as_ref()?
            .get(self.external_index)?
            .clone();
        self.external_index += 1;
        Some(answer)
    }

    pub(crate) fn external_predicate(&self) -> Option<PredicateKey> {
        self.external_predicate.clone()
    }

    pub(crate) fn external_tail_call(&self) -> bool {
        self.external_tail_call
    }

    pub(crate) fn has_external_answers(&self) -> bool {
        self.external_answers
            .as_ref()
            .is_some_and(|answers| self.external_index < answers.len())
    }
}
