use super::error::{WamError, WamResult};

#[derive(Debug, Clone)]
pub struct RegisterFile {
    registers: Vec<Option<usize>>,
}

impl RegisterFile {
    pub fn new(size: usize) -> Self {
        Self {
            registers: vec![None; size],
        }
    }

    pub fn set(&mut self, index: usize, addr: usize) -> WamResult<()> {
        self.ensure(index)?;
        self.registers[index] = Some(addr);
        Ok(())
    }

    pub fn get(&self, index: usize) -> WamResult<usize> {
        self.ensure(index)?;
        self.registers[index].ok_or(WamError::AddressOutOfBounds(index))
    }

    pub fn clear(&mut self) {
        self.registers.fill(None);
    }

    pub fn snapshot(&self) -> Vec<Option<usize>> {
        self.registers.clone()
    }

    pub fn restore(&mut self, snapshot: Vec<Option<usize>>) {
        self.registers = snapshot;
    }

    fn ensure(&self, index: usize) -> WamResult<()> {
        if index < self.registers.len() {
            Ok(())
        } else {
            Err(WamError::AddressOutOfBounds(index))
        }
    }
}
