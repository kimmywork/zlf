use super::cell::Cell;
use super::error::{WamError, WamResult};
use super::machine::M0Machine;
use super::register::RegisterFile;
use super::structure_mode::StructureMode;

pub(crate) fn put_structure(
    machine: &mut M0Machine,
    registers: &mut RegisterFile,
    mode: &mut StructureMode,
    name: &str,
    arity: usize,
    register: usize,
) -> WamResult<bool> {
    let addr = machine.put_structure(name, arity);
    registers.set(register, addr)?;
    *mode = StructureMode::write(arity);
    Ok(true)
}

pub(crate) fn get_structure(
    machine: &mut M0Machine,
    registers: &mut RegisterFile,
    mode: &mut StructureMode,
    name: &str,
    arity: usize,
    register: usize,
) -> WamResult<bool> {
    let addr = registers.get(register)?;
    let deref = machine.heap().deref(addr)?;
    match machine.heap().cell(deref)?.clone() {
        Cell::Ref(_) => bind_new_structure(machine, mode, addr, name, arity),
        Cell::Str(_) => read_existing_structure(machine, mode, deref, name, arity),
        _ => Ok(false),
    }
}

pub(crate) fn unify_constant(
    machine: &mut M0Machine,
    mode: &mut StructureMode,
    value: &str,
) -> WamResult<bool> {
    match mode.clone() {
        StructureMode::Read { .. } => unify_read_constant(machine, mode, value),
        StructureMode::Write { .. } => write_constant_arg(machine, mode, value),
        StructureMode::None => Err(WamError::InvalidInstructionState("unify_constant")),
    }
}

pub(crate) fn unify_variable(
    machine: &mut M0Machine,
    registers: &mut RegisterFile,
    mode: &mut StructureMode,
    register: usize,
) -> WamResult<bool> {
    match mode.clone() {
        StructureMode::Read { .. } => read_variable_arg(registers, mode, register),
        StructureMode::Write { .. } => write_variable_arg(machine, registers, mode, register),
        StructureMode::None => Err(WamError::InvalidInstructionState("unify_variable")),
    }
}

pub(crate) fn unify_value(
    machine: &mut M0Machine,
    registers: &mut RegisterFile,
    mode: &mut StructureMode,
    register: usize,
) -> WamResult<bool> {
    match mode.clone() {
        StructureMode::Read { .. } => unify_read_value(machine, registers, mode, register),
        StructureMode::Write { .. } => write_value_arg(machine, registers, mode, register),
        StructureMode::None => Err(WamError::InvalidInstructionState("unify_value")),
    }
}

fn bind_new_structure(
    machine: &mut M0Machine,
    mode: &mut StructureMode,
    addr: usize,
    name: &str,
    arity: usize,
) -> WamResult<bool> {
    let structure = machine.put_structure(name, arity);
    let success = machine.unify(addr, structure)?;
    *mode = StructureMode::write(arity);
    Ok(success)
}

fn read_existing_structure(
    machine: &M0Machine,
    mode: &mut StructureMode,
    addr: usize,
    name: &str,
    arity: usize,
) -> WamResult<bool> {
    let (existing_name, existing_arity, first_arg) = machine.heap().structure_parts(addr)?;
    if existing_name != name || existing_arity != arity {
        return Ok(false);
    }
    *mode = StructureMode::read(first_arg, arity);
    Ok(true)
}

fn unify_read_constant(
    machine: &mut M0Machine,
    mode: &mut StructureMode,
    value: &str,
) -> WamResult<bool> {
    let arg = mode.next_read_arg()?;
    let constant = machine.put_constant(value);
    machine.unify(arg, constant)
}

fn write_constant_arg(
    machine: &mut M0Machine,
    mode: &mut StructureMode,
    value: &str,
) -> WamResult<bool> {
    machine.put_constant(value);
    mode.maybe_consume_write_arg()?;
    Ok(true)
}

fn read_variable_arg(
    registers: &mut RegisterFile,
    mode: &mut StructureMode,
    register: usize,
) -> WamResult<bool> {
    let arg = mode.next_read_arg()?;
    registers.set(register, arg)?;
    Ok(true)
}

fn write_variable_arg(
    machine: &mut M0Machine,
    registers: &mut RegisterFile,
    mode: &mut StructureMode,
    register: usize,
) -> WamResult<bool> {
    let addr = machine.set_variable();
    registers.set(register, addr)?;
    mode.maybe_consume_write_arg()?;
    Ok(true)
}

fn unify_read_value(
    machine: &mut M0Machine,
    registers: &RegisterFile,
    mode: &mut StructureMode,
    register: usize,
) -> WamResult<bool> {
    let arg = mode.next_read_arg()?;
    let value = registers.get(register)?;
    machine.unify(arg, value)
}

fn write_value_arg(
    machine: &mut M0Machine,
    registers: &RegisterFile,
    mode: &mut StructureMode,
    register: usize,
) -> WamResult<bool> {
    let addr = registers.get(register)?;
    machine.set_value(addr)?;
    mode.maybe_consume_write_arg()?;
    Ok(true)
}
