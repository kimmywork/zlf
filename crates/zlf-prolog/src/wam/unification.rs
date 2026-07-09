use super::cell::Cell;
use super::error::WamResult;
use super::heap::Heap;
use super::trail::Trail;

#[derive(Debug, Default)]
pub struct Unifier {
    pdl: Vec<(usize, usize)>,
    trail: Trail,
}

impl Unifier {
    pub fn new() -> Self {
        Self {
            pdl: Vec::new(),
            trail: Trail::new(),
        }
    }

    pub fn unify(&mut self, heap: &mut Heap, left: usize, right: usize) -> WamResult<bool> {
        let checkpoint = self.trail.checkpoint();
        self.pdl.clear();
        self.pdl.push((left, right));
        while let Some((a, b)) = self.pdl.pop() {
            if !self.unify_pair(heap, a, b)? {
                self.trail.unwind(heap, checkpoint)?;
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn trail_checkpoint(&self) -> usize {
        self.trail.checkpoint()
    }

    pub fn unwind(&mut self, heap: &mut Heap, checkpoint: usize) -> WamResult<()> {
        self.trail.unwind(heap, checkpoint)
    }

    fn unify_pair(&mut self, heap: &mut Heap, left: usize, right: usize) -> WamResult<bool> {
        let a = heap.deref(left)?;
        let b = heap.deref(right)?;
        if a == b {
            return Ok(true);
        }
        match (heap.cell(a)?.clone(), heap.cell(b)?.clone()) {
            (Cell::Ref(_), _) if heap.is_unbound_ref(a)? => self.bind(heap, a, b),
            (_, Cell::Ref(_)) if heap.is_unbound_ref(b)? => self.bind(heap, b, a),
            (Cell::Constant(x), Cell::Constant(y)) => Ok(x == y),
            (Cell::Str(_), Cell::Str(_)) => self.unify_structures(heap, a, b),
            _ => Ok(false),
        }
    }

    fn bind(&mut self, heap: &mut Heap, var_addr: usize, value_addr: usize) -> WamResult<bool> {
        self.trail.trail_ref(var_addr);
        heap.bind_ref(var_addr, value_addr)?;
        Ok(true)
    }

    fn unify_structures(&mut self, heap: &Heap, left: usize, right: usize) -> WamResult<bool> {
        let (left_name, left_arity, left_args) = heap.structure_parts(left)?;
        let (right_name, right_arity, right_args) = heap.structure_parts(right)?;
        if left_name != right_name || left_arity != right_arity {
            return Ok(false);
        }
        for offset in 0..left_arity {
            self.pdl.push((left_args + offset, right_args + offset));
        }
        Ok(true)
    }
}
