use crate::parser::Term;

use super::error::WamResult;
use super::executor::WamExecutor;

impl WamExecutor {
    pub(crate) fn execute_list_builtin(
        &mut self,
        name: &str,
        arity: usize,
    ) -> WamResult<Option<bool>> {
        let result = match (name, arity) {
            ("length", 2) => self.eval_length()?,
            ("nth0", 3) => self.eval_nth(0)?,
            ("nth1", 3) => self.eval_nth(1)?,
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    fn eval_length(&mut self) -> WamResult<bool> {
        let list = self.register_term(0)?;
        if let Some(items) = list_items(&list) {
            return self.unify_register_term(1, &Term::Integer(items.len() as i64));
        }
        let Term::Integer(length) = self.register_term(1)? else {
            return Ok(false);
        };
        if length < 0 {
            return Ok(false);
        }
        let list = Term::List(
            (0..length)
                .map(|_| Term::Variable("_".to_string()))
                .collect(),
        );
        self.unify_register_term(0, &list)
    }

    fn eval_nth(&mut self, base: i64) -> WamResult<bool> {
        let Term::Integer(index) = self.register_term(0)? else {
            return Ok(false);
        };
        let list = self.register_term(1)?;
        let Some(items) = list_items(&list) else {
            return Ok(false);
        };
        let offset = index - base;
        if offset < 0 || offset as usize >= items.len() {
            return Ok(false);
        }
        self.unify_register_term(2, &items[offset as usize])
    }
}

fn list_items(term: &Term) -> Option<&[Term]> {
    match term {
        Term::List(items) => Some(items),
        Term::Atom(name) if name == "[]" => Some(&[]),
        _ => None,
    }
}
