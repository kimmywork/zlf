use std::collections::HashMap;

use crate::parser::Term;

use super::cell::Cell;
use super::error::{WamError, WamResult};
use super::heap::Heap;

pub fn term_from_heap(heap: &Heap, addr: usize) -> WamResult<Term> {
    term_from_heap_with_variables(heap, addr, &mut HashMap::new())
}

fn term_from_heap_with_variables(
    heap: &Heap,
    addr: usize,
    variables: &mut HashMap<usize, String>,
) -> WamResult<Term> {
    let deref = heap.deref(addr)?;
    match heap.cell(deref)? {
        Cell::Constant(value) => Ok(super::constant::decode(value)),
        Cell::Str(_) => structure_from_heap(heap, deref, variables),
        Cell::Ref(_) => {
            let next = variables.len();
            let name = variables
                .entry(deref)
                .or_insert_with(|| format!("_G{next}"));
            Ok(Term::Variable(name.clone()))
        }
        Cell::Functor { .. } => Err(WamError::ExpectedFunctor(deref)),
    }
}

fn structure_from_heap(
    heap: &Heap,
    addr: usize,
    variables: &mut HashMap<usize, String>,
) -> WamResult<Term> {
    let (name, arity, first_arg) = heap.structure_parts(addr)?;
    let args = (0..arity)
        .map(|offset| term_from_heap_with_variables(heap, first_arg + offset, variables))
        .collect::<WamResult<Vec<_>>>()?;
    if name == "." && args.len() == 2 {
        list_from_cons(args)
    } else if name == "list" {
        Ok(Term::List(args))
    } else if name == "object" {
        object_from_args(args)
    } else {
        Ok(Term::Compound {
            name: name.to_string(),
            args,
        })
    }
}

fn list_from_cons(mut args: Vec<Term>) -> WamResult<Term> {
    let head = args.remove(0);
    let tail = args.remove(0);
    match tail {
        Term::List(mut items) => {
            items.insert(0, head);
            Ok(Term::List(items))
        }
        tail => Ok(Term::Compound {
            name: ".".to_string(),
            args: vec![head, tail],
        }),
    }
}

fn object_from_args(args: Vec<Term>) -> WamResult<Term> {
    let mut entries = Vec::new();
    for arg in args {
        match arg {
            Term::Compound { name, args } if name == "pair" && args.len() == 2 => {
                if let Term::Atom(key) = &args[0] {
                    entries.push((key.clone(), args[1].clone()));
                } else {
                    return Err(WamError::UnsupportedTerm("object key"));
                }
            }
            _ => return Err(WamError::UnsupportedTerm("object pair")),
        }
    }
    Ok(Term::Object(entries))
}
