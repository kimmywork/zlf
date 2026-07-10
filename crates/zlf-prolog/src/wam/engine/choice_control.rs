use super::choice_point::ChoicePointFrame;
use super::environment_stack::EnvironmentStack;
use super::error::{WamError, WamResult};
use super::machine::M0Machine;
use super::register::RegisterFile;

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_me_else(
    stack: &mut Vec<ChoicePointFrame>,
    machine: &M0Machine,
    registers: &RegisterFile,
    environments: &EnvironmentStack,
    call_stack: &[usize],
    cut_base_stack: &[usize],
    next: usize,
    proof_checkpoint: usize,
) {
    let frame = ChoicePointFrame::capture_with_call_stack(
        machine,
        registers,
        environments,
        call_stack,
        cut_base_stack,
        None,
        next,
        proof_checkpoint,
    );
    stack.push(frame);
}

pub(crate) fn retry_me_else(
    stack: &mut [ChoicePointFrame],
    machine: &mut M0Machine,
    registers: &mut RegisterFile,
    environments: &mut EnvironmentStack,
    call_stack: &mut Vec<usize>,
    cut_base_stack: &mut Vec<usize>,
    next: usize,
) -> WamResult<usize> {
    let frame = stack
        .last_mut()
        .ok_or(WamError::InvalidInstructionState("retry_me_else"))?;
    frame.restore(machine, registers, environments)?;
    *call_stack = frame.call_stack();
    *cut_base_stack = frame.cut_base_stack();
    let proof_checkpoint = frame.proof_checkpoint();
    frame.retarget(next);
    Ok(proof_checkpoint)
}

pub(crate) fn trust_me(
    stack: &mut Vec<ChoicePointFrame>,
    machine: &mut M0Machine,
    registers: &mut RegisterFile,
    environments: &mut EnvironmentStack,
    call_stack: &mut Vec<usize>,
    cut_base_stack: &mut Vec<usize>,
) -> WamResult<usize> {
    let frame = stack
        .pop()
        .ok_or(WamError::InvalidInstructionState("trust_me"))?;
    frame.restore(machine, registers, environments)?;
    *call_stack = frame.call_stack();
    *cut_base_stack = frame.cut_base_stack();
    Ok(frame.proof_checkpoint())
}
