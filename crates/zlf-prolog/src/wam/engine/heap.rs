use super::cell::Cell;
use super::error::{WamError, WamResult};

#[derive(Debug, Default, Clone)]
pub struct Heap {
    cells: Vec<Cell>,
}

impl Heap {
    pub fn new() -> Self {
        Self { cells: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cell(&self, addr: usize) -> WamResult<&Cell> {
        self.cells
            .get(addr)
            .ok_or(WamError::AddressOutOfBounds(addr))
    }

    pub fn bind_ref(&mut self, var_addr: usize, value_addr: usize) -> WamResult<()> {
        self.ensure_addr(var_addr)?;
        self.cells[var_addr] = Cell::Ref(value_addr);
        Ok(())
    }

    pub fn reset_ref(&mut self, addr: usize) -> WamResult<()> {
        self.ensure_addr(addr)?;
        self.cells[addr] = Cell::Ref(addr);
        Ok(())
    }

    pub fn checkpoint(&self) -> usize {
        self.cells.len()
    }

    pub fn unwind(&mut self, checkpoint: usize) -> WamResult<()> {
        if checkpoint <= self.cells.len() {
            self.cells.truncate(checkpoint);
            Ok(())
        } else {
            Err(WamError::AddressOutOfBounds(checkpoint))
        }
    }

    pub fn put_structure(&mut self, name: impl Into<String>, arity: usize) -> usize {
        let str_addr = self.cells.len();
        let functor_addr = str_addr + 1;
        self.cells.push(Cell::Str(functor_addr));
        self.cells.push(Cell::functor(name, arity));
        str_addr
    }

    pub fn put_constant(&mut self, value: impl Into<String>) -> usize {
        let addr = self.cells.len();
        self.cells.push(Cell::Constant(value.into()));
        addr
    }

    pub fn set_variable(&mut self) -> usize {
        let addr = self.cells.len();
        self.cells.push(Cell::Ref(addr));
        addr
    }

    pub fn set_value(&mut self, addr: usize) -> WamResult<usize> {
        let cell = self.cell(addr)?.clone();
        let new_addr = self.cells.len();
        self.cells.push(cell);
        Ok(new_addr)
    }

    pub fn deref(&self, mut addr: usize) -> WamResult<usize> {
        loop {
            match self.cell(addr)? {
                Cell::Ref(target) if *target != addr => addr = *target,
                _ => return Ok(addr),
            }
        }
    }

    pub fn structure_parts(&self, str_addr: usize) -> WamResult<(&str, usize, usize)> {
        let functor_addr = match self.cell(str_addr)? {
            Cell::Str(addr) => *addr,
            _ => return Err(WamError::ExpectedFunctor(str_addr)),
        };
        match self.cell(functor_addr)? {
            Cell::Functor { name, arity } => Ok((name, *arity, functor_addr + 1)),
            _ => Err(WamError::ExpectedFunctor(functor_addr)),
        }
    }

    pub fn is_unbound_ref(&self, addr: usize) -> WamResult<bool> {
        Ok(self.cell(addr)?.is_unbound_ref_at(addr))
    }

    fn ensure_addr(&self, addr: usize) -> WamResult<()> {
        self.cell(addr).map(|_| ())
    }
}
