use super::choice_point::ChoicePointFrame;
use super::error::{WamError, WamResult};
use super::machine::M0Machine;
use super::register::RegisterFile;

pub(crate) fn try_me_else(
    stack: &mut Vec<ChoicePointFrame>,
    machine: &M0Machine,
    registers: &RegisterFile,
    next: usize,
) {
    let frame = ChoicePointFrame::capture(machine, registers, None, next);
    stack.push(frame);
}

pub(crate) fn retry_me_else(
    stack: &mut [ChoicePointFrame],
    machine: &mut M0Machine,
    registers: &mut RegisterFile,
    next: usize,
) -> WamResult<()> {
    let frame = stack
        .last_mut()
        .ok_or(WamError::InvalidInstructionState("retry_me_else"))?;
    frame.restore(machine, registers)?;
    frame.retarget(next);
    Ok(())
}

pub(crate) fn trust_me(
    stack: &mut Vec<ChoicePointFrame>,
    machine: &mut M0Machine,
    registers: &mut RegisterFile,
) -> WamResult<()> {
    let frame = stack
        .pop()
        .ok_or(WamError::InvalidInstructionState("trust_me"))?;
    frame.restore(machine, registers)
}
