use super::cell::Cell;
use super::error::WamResult;
use super::execution_result::StepOutcome;
use super::executor::WamExecutor;

use super::instruction::Instruction;

impl WamExecutor {
    pub(crate) fn switch_step(&self, instruction: &Instruction) -> WamResult<Option<StepOutcome>> {
        match instruction {
            Instruction::SwitchOnTerm {
                register,
                variable,
                constant,
                list,
                structure,
            } => self
                .switch_on_term(*register, *variable, *constant, *list, *structure)
                .map(Some),
            Instruction::SwitchOnConstant {
                register,
                cases,
                default,
            } => self
                .switch_on_constant(*register, cases, *default)
                .map(Some),
            Instruction::SwitchOnStructure {
                register,
                cases,
                default,
            } => self
                .switch_on_structure(*register, cases, *default)
                .map(Some),
            _ => Ok(None),
        }
    }

    pub(crate) fn switch_on_term(
        &self,
        register: usize,
        variable: Option<usize>,
        constant: Option<usize>,
        list: Option<usize>,
        structure: Option<usize>,
    ) -> WamResult<StepOutcome> {
        let addr = self.registers.get(register)?;
        let deref = self.machine.heap().deref(addr)?;
        let target = match self.machine.heap().cell(deref)? {
            Cell::Ref(_) => variable,
            Cell::Constant(_) => constant,
            Cell::Str(_) => self.structure_term_target(deref, list, structure)?,
            Cell::Functor { .. } => None,
        };
        Ok(target.map_or(StepOutcome::Failed, StepOutcome::Jump))
    }

    pub(crate) fn switch_on_constant(
        &self,
        register: usize,
        cases: &[(String, usize)],
        default: Option<usize>,
    ) -> WamResult<StepOutcome> {
        let addr = self.registers.get(register)?;
        let deref = self.machine.heap().deref(addr)?;
        let target = match self.machine.heap().cell(deref)? {
            Cell::Constant(value) => cases
                .iter()
                .find_map(|(case, target)| (case == value).then_some(*target))
                .or(default),
            _ => default,
        };
        Ok(target.map_or(StepOutcome::Failed, StepOutcome::Jump))
    }

    pub(crate) fn switch_on_structure(
        &self,
        register: usize,
        cases: &[(String, usize, usize)],
        default: Option<usize>,
    ) -> WamResult<StepOutcome> {
        let addr = self.registers.get(register)?;
        let deref = self.machine.heap().deref(addr)?;
        let target = match self.machine.heap().cell(deref)? {
            Cell::Str(_) => self.structure_case_target(deref, cases)?.or(default),
            _ => default,
        };
        Ok(target.map_or(StepOutcome::Failed, StepOutcome::Jump))
    }

    fn structure_term_target(
        &self,
        addr: usize,
        list: Option<usize>,
        structure: Option<usize>,
    ) -> WamResult<Option<usize>> {
        let (name, _, _) = self.machine.heap().structure_parts(addr)?;
        Ok(if name == "list" { list } else { structure })
    }

    fn structure_case_target(
        &self,
        addr: usize,
        cases: &[(String, usize, usize)],
    ) -> WamResult<Option<usize>> {
        let (name, arity, _) = self.machine.heap().structure_parts(addr)?;
        Ok(cases.iter().find_map(|(case, case_arity, target)| {
            (case == name && *case_arity == arity).then_some(*target)
        }))
    }
}
