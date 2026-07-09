use super::error::WamResult;
use super::executor::WamExecutor;
use super::structure_mode::StructureMode;
use super::structure_ops;

impl WamExecutor {
    pub(crate) fn put_variable(&mut self, register: usize) -> WamResult<bool> {
        let addr = self.machine.set_variable();
        self.registers.set(register, addr)?;
        Ok(true)
    }

    pub(crate) fn put_value(&mut self, source: usize, target: usize) -> WamResult<bool> {
        let addr = self.registers.get(source)?;
        self.registers.set(target, addr)?;
        Ok(true)
    }

    pub(crate) fn put_permanent_value(&mut self, slot: usize, register: usize) -> WamResult<bool> {
        let addr = self.permanent_or_new(slot)?;
        self.registers.set(register, addr)?;
        Ok(true)
    }

    pub(crate) fn put_constant(&mut self, value: &str, register: usize) -> WamResult<bool> {
        let addr = self.machine.put_constant(value);
        self.registers.set(register, addr)?;
        Ok(true)
    }

    pub(crate) fn put_structure(
        &mut self,
        name: &str,
        arity: usize,
        register: usize,
    ) -> WamResult<bool> {
        structure_ops::put_structure(
            &mut self.machine,
            &mut self.registers,
            &mut self.mode,
            name,
            arity,
            register,
        )
    }

    pub(crate) fn put_list(&mut self, arity: usize, register: usize) -> WamResult<bool> {
        self.put_structure("list", arity, register)
    }

    pub(crate) fn set_variable(&mut self, register: usize) -> WamResult<bool> {
        let addr = self.machine.set_variable();
        self.registers.set(register, addr)?;
        self.mode.maybe_consume_write_arg()?;
        Ok(true)
    }

    pub(crate) fn set_value(&mut self, register: usize) -> WamResult<bool> {
        let addr = self.registers.get(register)?;
        self.machine.set_value(addr)?;
        self.mode.maybe_consume_write_arg()?;
        Ok(true)
    }

    pub(crate) fn set_permanent_value(&mut self, slot: usize) -> WamResult<bool> {
        let addr = self.permanent_or_new(slot)?;
        self.machine.set_value(addr)?;
        self.mode.maybe_consume_write_arg()?;
        Ok(true)
    }

    pub(crate) fn set_constant(&mut self, value: &str) -> WamResult<bool> {
        self.machine.put_constant(value);
        self.mode.maybe_consume_write_arg()?;
        Ok(true)
    }

    pub(crate) fn get_constant(&mut self, value: &str, register: usize) -> WamResult<bool> {
        let constant = self.machine.put_constant(value);
        let addr = self.registers.get(register)?;
        self.machine.unify(addr, constant)
    }

    pub(crate) fn get_permanent_value(&mut self, slot: usize, register: usize) -> WamResult<bool> {
        let addr = self.registers.get(register)?;
        if let Some(value) = self.environments.permanent_slot(slot)? {
            self.machine.unify(value, addr)
        } else {
            self.environments.set_permanent_slot(slot, addr)?;
            Ok(true)
        }
    }

    pub(crate) fn get_structure(
        &mut self,
        name: &str,
        arity: usize,
        register: usize,
    ) -> WamResult<bool> {
        structure_ops::get_structure(
            &mut self.machine,
            &mut self.registers,
            &mut self.mode,
            name,
            arity,
            register,
        )
    }

    pub(crate) fn get_list(&mut self, arity: usize, register: usize) -> WamResult<bool> {
        self.get_structure("list", arity, register)
    }

    pub(crate) fn unify_constant(&mut self, value: &str) -> WamResult<bool> {
        structure_ops::unify_constant(&mut self.machine, &mut self.mode, value)
    }

    pub(crate) fn unify_variable(&mut self, register: usize) -> WamResult<bool> {
        structure_ops::unify_variable(
            &mut self.machine,
            &mut self.registers,
            &mut self.mode,
            register,
        )
    }

    pub(crate) fn unify_value(&mut self, register: usize) -> WamResult<bool> {
        structure_ops::unify_value(
            &mut self.machine,
            &mut self.registers,
            &mut self.mode,
            register,
        )
    }

    pub(crate) fn unify_permanent_value(&mut self, slot: usize) -> WamResult<bool> {
        let addr = self.environments.permanent_slot(slot)?;
        match (self.mode.clone(), addr) {
            (StructureMode::Read { .. }, Some(value)) => {
                let arg = self.mode.next_read_arg()?;
                self.machine.unify(arg, value)
            }
            (StructureMode::Read { .. }, None) => {
                let arg = self.mode.next_read_arg()?;
                self.environments.set_permanent_slot(slot, arg)?;
                Ok(true)
            }
            (StructureMode::Write { .. }, Some(value)) => {
                self.machine.set_value(value)?;
                self.mode.consume_write_arg()?;
                Ok(true)
            }
            (StructureMode::Write { .. }, None) => {
                let value = self.machine.set_variable();
                self.environments.set_permanent_slot(slot, value)?;
                self.mode.consume_write_arg()?;
                Ok(true)
            }
            (StructureMode::None, _) => Err(super::error::WamError::InvalidInstructionState(
                "unify_permanent_value",
            )),
        }
    }

    pub(crate) fn unify_registers(&mut self, left: usize, right: usize) -> WamResult<bool> {
        let left_addr = self.registers.get(left)?;
        let right_addr = self.registers.get(right)?;
        self.machine.unify(left_addr, right_addr)
    }

    pub(crate) fn permanent_or_new(&mut self, slot: usize) -> WamResult<usize> {
        if let Some(addr) = self.environments.permanent_slot(slot)? {
            Ok(addr)
        } else {
            let addr = self.machine.set_variable();
            self.environments.set_permanent_slot(slot, addr)?;
            Ok(addr)
        }
    }
}
