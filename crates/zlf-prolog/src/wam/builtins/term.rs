use std::cmp::Ordering;

use crate::parser::Term;

use super::cell::Cell;
use super::error::WamResult;
use super::executor::WamExecutor;

impl WamExecutor {
    pub(crate) fn execute_term_builtin(
        &mut self,
        name: &str,
        arity: usize,
    ) -> WamResult<Option<bool>> {
        let result = match (name, arity) {
            ("=", 2) => self.unify_registers(0, 1)?,
            ("\\=", 2) => !self.registers_unifiable()?,
            ("==", 2) => self.registers_identical()?,
            ("\\==", 2) => !self.registers_identical()?,
            ("@<", 2) => self.compare_register_terms()?.is_lt(),
            ("@=<", 2) => self.compare_register_terms()?.is_le(),
            ("@>", 2) => self.compare_register_terms()?.is_gt(),
            ("@>=", 2) => self.compare_register_terms()?.is_ge(),
            ("var", 1) => self.register_is_variable(0)?,
            ("nonvar", 1) => !self.register_is_variable(0)?,
            ("atom", 1) => matches!(self.register_term(0)?, Term::Atom(_)),
            ("integer", 1) => matches!(self.register_term(0)?, Term::Integer(_)),
            ("float", 1) => matches!(self.register_term(0)?, Term::Float(_)),
            ("number", 1) => matches!(self.register_term(0)?, Term::Integer(_) | Term::Float(_)),
            ("atomic", 1) => matches!(
                self.register_term(0)?,
                Term::Atom(_) | Term::Integer(_) | Term::Float(_) | Term::String(_)
            ),
            ("compound", 1) => matches!(
                self.register_term(0)?,
                Term::Compound { .. } | Term::List(_) | Term::Object(_)
            ),
            ("ground", 1) => is_ground(&self.register_term(0)?),
            ("functor", 3) => self.eval_functor()?,
            ("arg", 3) => self.eval_arg()?,
            ("=..", 2) => self.eval_univ()?,
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    fn registers_unifiable(&mut self) -> WamResult<bool> {
        let heap = self.machine.heap_checkpoint();
        let trail = self.machine.trail_checkpoint();
        let result = self.unify_registers(0, 1)?;
        self.machine.unwind_trail(trail)?;
        self.machine.unwind_heap(heap)?;
        Ok(result)
    }

    fn registers_identical(&self) -> WamResult<bool> {
        self.identical_addresses(self.registers.get(0)?, self.registers.get(1)?)
    }

    fn identical_addresses(&self, left: usize, right: usize) -> WamResult<bool> {
        let left = self.machine.heap().deref(left)?;
        let right = self.machine.heap().deref(right)?;
        if left == right {
            return Ok(true);
        }
        match (
            self.machine.heap().cell(left)?,
            self.machine.heap().cell(right)?,
        ) {
            (Cell::Constant(left), Cell::Constant(right)) => Ok(left == right),
            (Cell::Str(_), Cell::Str(_)) => self.identical_structures(left, right),
            _ => Ok(false),
        }
    }

    fn identical_structures(&self, left: usize, right: usize) -> WamResult<bool> {
        let (left_name, left_arity, left_args) = self.machine.heap().structure_parts(left)?;
        let (right_name, right_arity, right_args) = self.machine.heap().structure_parts(right)?;
        if left_name != right_name || left_arity != right_arity {
            return Ok(false);
        }
        for offset in 0..left_arity {
            if !self.identical_addresses(left_args + offset, right_args + offset)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn register_is_variable(&self, register: usize) -> WamResult<bool> {
        let address = self.registers.get(register)?;
        let address = self.machine.heap().deref(address)?;
        self.machine.heap().is_unbound_ref(address)
    }

    fn compare_register_terms(&self) -> WamResult<Ordering> {
        self.compare_addresses(self.registers.get(0)?, self.registers.get(1)?)
    }

    fn compare_addresses(&self, left: usize, right: usize) -> WamResult<Ordering> {
        let left = self.machine.heap().deref(left)?;
        let right = self.machine.heap().deref(right)?;
        let left_cell = self.machine.heap().cell(left)?;
        let right_cell = self.machine.heap().cell(right)?;
        let rank = cell_rank(left_cell).cmp(&cell_rank(right_cell));
        if !rank.is_eq() {
            return Ok(rank);
        }
        match (left_cell, right_cell) {
            (Cell::Ref(_), Cell::Ref(_)) => Ok(left.cmp(&right)),
            (Cell::Constant(left), Cell::Constant(right)) => Ok(compare_terms(
                &super::constant::decode(left),
                &super::constant::decode(right),
            )),
            (Cell::Str(_), Cell::Str(_)) => self.compare_structures(left, right),
            _ => Ok(Ordering::Equal),
        }
    }

    fn compare_structures(&self, left: usize, right: usize) -> WamResult<Ordering> {
        let (left_name, left_arity, left_args) = self.machine.heap().structure_parts(left)?;
        let (right_name, right_arity, right_args) = self.machine.heap().structure_parts(right)?;
        let functor = left_arity
            .cmp(&right_arity)
            .then_with(|| left_name.cmp(right_name));
        if !functor.is_eq() {
            return Ok(functor);
        }
        for offset in 0..left_arity {
            let ordering = self.compare_addresses(left_args + offset, right_args + offset)?;
            if !ordering.is_eq() {
                return Ok(ordering);
            }
        }
        Ok(Ordering::Equal)
    }

    fn eval_functor(&mut self) -> WamResult<bool> {
        let term = self.register_term(0)?;
        if !matches!(term, Term::Variable(_)) {
            let (name, arity) = functor_parts(&term);
            return Ok(self.unify_register_term(1, &name)?
                && self.unify_register_term(2, &Term::Integer(arity as i64))?);
        }
        let name = self.register_term(1)?;
        let Term::Integer(arity) = self.register_term(2)? else {
            return Ok(false);
        };
        if arity < 0 {
            return Ok(false);
        }
        let built = build_functor(name, arity as usize)?;
        self.unify_register_term(0, &built)
    }

    fn eval_arg(&mut self) -> WamResult<bool> {
        let Term::Integer(index) = self.register_term(0)? else {
            return Ok(false);
        };
        if index < 1 {
            return Ok(false);
        }
        let address = self.machine.heap().deref(self.registers.get(1)?)?;
        let Ok((_, arity, first_arg)) = self.machine.heap().structure_parts(address) else {
            return Ok(false);
        };
        let index = index as usize;
        if index > arity {
            return Ok(false);
        }
        let output = self.registers.get(2)?;
        self.machine.unify(output, first_arg + index - 1)
    }

    fn eval_univ(&mut self) -> WamResult<bool> {
        let term = self.register_term(0)?;
        if !matches!(term, Term::Variable(_)) {
            let (name, args) = univ_parts(term);
            let mut items = vec![name];
            items.extend(args);
            return self.unify_register_term(1, &Term::List(items));
        }
        let Term::List(items) = self.register_term(1)? else {
            return Ok(false);
        };
        let Some((name, args)) = items.split_first() else {
            return Ok(false);
        };
        let built = if args.is_empty() {
            name.clone()
        } else {
            let Term::Atom(name) = name else {
                return Ok(false);
            };
            Term::Compound {
                name: name.clone(),
                args: args.to_vec(),
            }
        };
        self.unify_register_term(0, &built)
    }

    pub(crate) fn unify_register_term(&mut self, register: usize, term: &Term) -> WamResult<bool> {
        let value = self.machine.put_term(term)?;
        self.machine.unify(self.registers.get(register)?, value)
    }
}

fn is_ground(term: &Term) -> bool {
    match term {
        Term::Variable(_) => false,
        Term::Compound { args, .. } | Term::List(args) => args.iter().all(is_ground),
        Term::Object(entries) => entries.iter().all(|(_, value)| is_ground(value)),
        _ => true,
    }
}

fn functor_parts(term: &Term) -> (Term, usize) {
    match term {
        Term::Compound { name, args } => (Term::Atom(name.clone()), args.len()),
        Term::List(items) if items.is_empty() => (Term::Atom("[]".to_string()), 0),
        Term::List(_) => (Term::Atom(".".to_string()), 2),
        atomic => (atomic.clone(), 0),
    }
}

fn build_functor(name: Term, arity: usize) -> WamResult<Term> {
    if arity == 0 {
        return Ok(name);
    }
    let Term::Atom(name) = name else {
        return Err(super::WamError::Provider(
            "functor name must be an atom".to_string(),
        ));
    };
    Ok(Term::Compound {
        name,
        args: (0..arity)
            .map(|_| Term::Variable("_".to_string()))
            .collect(),
    })
}

fn univ_parts(term: Term) -> (Term, Vec<Term>) {
    match term {
        Term::Compound { name, args } => (Term::Atom(name), args),
        Term::List(items) if items.is_empty() => (Term::Atom("[]".to_string()), Vec::new()),
        Term::List(mut items) => {
            let head = items.remove(0);
            (Term::Atom(".".to_string()), vec![head, Term::List(items)])
        }
        atomic => (atomic, Vec::new()),
    }
}

fn cell_rank(cell: &Cell) -> u8 {
    match cell {
        Cell::Ref(_) => 0,
        Cell::Constant(value) => term_rank(&super::constant::decode(value)),
        Cell::Str(_) | Cell::Functor { .. } => 3,
    }
}

fn compare_terms(left: &Term, right: &Term) -> Ordering {
    let rank = term_rank(left).cmp(&term_rank(right));
    if !rank.is_eq() {
        return rank;
    }
    match (left, right) {
        (Term::Integer(left), Term::Integer(right)) => left.cmp(right),
        (Term::Float(left), Term::Float(right)) => left.total_cmp(right),
        (Term::Integer(left), Term::Float(right)) => (*left as f64).total_cmp(right),
        (Term::Float(left), Term::Integer(right)) => left.total_cmp(&(*right as f64)),
        _ => term_key(left).cmp(&term_key(right)),
    }
}

fn term_rank(term: &Term) -> u8 {
    match term {
        Term::Variable(_) => 0,
        Term::Integer(_) | Term::Float(_) => 1,
        Term::Atom(_) | Term::String(_) => 2,
        Term::Compound { .. } | Term::List(_) | Term::Object(_) => 3,
    }
}

fn term_key(term: &Term) -> String {
    match term {
        Term::Variable(name) | Term::Atom(name) | Term::String(name) => name.clone(),
        Term::Integer(value) => format!("{value:020}"),
        Term::Float(value) => format!("{value:030.12}"),
        Term::Compound { name, args } => format!("{}:{name}:{args:?}", args.len()),
        Term::List(items) => format!("2:.:{items:?}"),
        Term::Object(entries) => format!("{}:object:{entries:?}", entries.len()),
    }
}
