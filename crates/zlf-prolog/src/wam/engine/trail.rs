use super::error::WamResult;
use super::heap::Heap;

#[derive(Debug, Default, Clone)]
pub struct Trail {
    entries: Vec<usize>,
}

impl Trail {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn checkpoint(&self) -> usize {
        self.entries.len()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn trail_ref(&mut self, addr: usize) {
        self.entries.push(addr);
    }

    pub fn unwind(&mut self, heap: &mut Heap, checkpoint: usize) -> WamResult<()> {
        while self.entries.len() > checkpoint {
            if let Some(addr) = self.entries.pop() {
                heap.reset_ref(addr)?;
            }
        }
        Ok(())
    }
}
