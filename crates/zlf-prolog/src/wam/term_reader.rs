use crate::parser::Term;

use super::cell::Cell;
use super::error::{WamError, WamResult};
use super::heap::Heap;

pub fn term_from_heap(heap: &Heap, addr: usize) -> WamResult<Term> {
    let deref = heap.deref(addr)?;
    match heap.cell(deref)? {
        Cell::Constant(value) => Ok(Term::Atom(value.clone())),
        Cell::Str(_) => structure_from_heap(heap, deref),
        Cell::Ref(_) => Ok(Term::Variable("_".to_string())),
        Cell::Functor { .. } => Err(WamError::ExpectedFunctor(deref)),
    }
}

fn structure_from_heap(heap: &Heap, addr: usize) -> WamResult<Term> {
    let (name, arity, first_arg) = heap.structure_parts(addr)?;
    let args = (0..arity)
        .map(|offset| term_from_heap(heap, first_arg + offset))
        .collect::<WamResult<Vec<_>>>()?;
    if name == "list" {
        Ok(Term::List(args))
    } else {
        Ok(Term::Compound {
            name: name.to_string(),
            args,
        })
    }
}
