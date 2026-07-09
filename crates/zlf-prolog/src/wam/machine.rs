use super::error::WamResult;
use super::heap::Heap;
use super::unification::Unifier;

#[derive(Debug, Default)]
pub struct M0Machine {
    heap: Heap,
    unifier: Unifier,
}

impl M0Machine {
    pub fn new() -> Self {
        Self {
            heap: Heap::new(),
            unifier: Unifier::new(),
        }
    }

    pub fn heap(&self) -> &Heap {
        &self.heap
    }

    pub fn put_structure(&mut self, name: impl Into<String>, arity: usize) -> usize {
        self.heap.put_structure(name, arity)
    }

    pub fn put_constant(&mut self, value: impl Into<String>) -> usize {
        self.heap.put_constant(value)
    }

    pub fn set_variable(&mut self) -> usize {
        self.heap.set_variable()
    }

    pub fn set_value(&mut self, addr: usize) -> WamResult<usize> {
        self.heap.set_value(addr)
    }

    pub fn unify(&mut self, left: usize, right: usize) -> WamResult<bool> {
        self.unifier.unify(&mut self.heap, left, right)
    }

    pub fn heap_checkpoint(&self) -> usize {
        self.heap.checkpoint()
    }

    pub fn unwind_heap(&mut self, checkpoint: usize) -> WamResult<()> {
        self.heap.unwind(checkpoint)
    }

    pub fn trail_checkpoint(&self) -> usize {
        self.unifier.trail_checkpoint()
    }

    pub fn unwind_trail(&mut self, checkpoint: usize) -> WamResult<()> {
        self.unifier.unwind(&mut self.heap, checkpoint)
    }
}
