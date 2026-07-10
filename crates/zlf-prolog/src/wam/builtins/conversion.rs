use crate::parser::Term;

use super::error::WamResult;
use super::executor::WamExecutor;

impl WamExecutor {
    pub(crate) fn execute_conversion_builtin(
        &mut self,
        name: &str,
        arity: usize,
    ) -> WamResult<Option<bool>> {
        if arity != 2 {
            return Ok(None);
        }
        let result = match name {
            "atom_string" => self.atom_string()?,
            "atom_chars" => self.text_list(true, false)?,
            "string_chars" => self.text_list(false, false)?,
            "atom_codes" => self.text_list(true, true)?,
            "string_codes" => self.text_list(false, true)?,
            "number_string" => self.number_string()?,
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    fn atom_string(&mut self) -> WamResult<bool> {
        match (self.register_term(0)?, self.register_term(1)?) {
            (Term::Atom(atom), _) => self.unify_register_term(1, &Term::String(atom)),
            (_, Term::String(string)) => self.unify_register_term(0, &Term::Atom(string)),
            _ => Ok(false),
        }
    }

    fn text_list(&mut self, atom_left: bool, codes: bool) -> WamResult<bool> {
        let left = self.register_term(0)?;
        let right = self.register_term(1)?;
        if let Some(text) = text_value(&left, atom_left) {
            let list = if codes {
                codes_list(&text)
            } else {
                chars_list(&text)
            };
            return self.unify_register_term(1, &list);
        }
        let Term::List(items) = right else {
            return Ok(false);
        };
        let Some(text) = list_text(&items, codes) else {
            return Ok(false);
        };
        let output = if atom_left {
            Term::Atom(text)
        } else {
            Term::String(text)
        };
        self.unify_register_term(0, &output)
    }

    fn number_string(&mut self) -> WamResult<bool> {
        match (self.register_term(0)?, self.register_term(1)?) {
            (number @ (Term::Integer(_) | Term::Float(_)), _) => {
                self.unify_register_term(1, &Term::String(number_text(&number)))
            }
            (_, Term::String(text)) => match parse_number(&text) {
                Some(number) => self.unify_register_term(0, &number),
                None => Ok(false),
            },
            _ => Ok(false),
        }
    }
}

fn text_value(term: &Term, atom: bool) -> Option<String> {
    match (atom, term) {
        (true, Term::Atom(text)) | (false, Term::String(text)) => Some(text.clone()),
        _ => None,
    }
}

fn chars_list(text: &str) -> Term {
    Term::List(text.chars().map(|ch| Term::Atom(ch.to_string())).collect())
}

fn codes_list(text: &str) -> Term {
    Term::List(
        text.chars()
            .map(|ch| Term::Integer(ch as u32 as i64))
            .collect(),
    )
}

fn list_text(items: &[Term], codes: bool) -> Option<String> {
    if codes {
        items
            .iter()
            .map(|term| match term {
                Term::Integer(code) => char::from_u32(*code as u32),
                _ => None,
            })
            .collect()
    } else {
        items
            .iter()
            .map(|term| match term {
                Term::Atom(ch) if ch.chars().count() == 1 => ch.chars().next(),
                _ => None,
            })
            .collect()
    }
}

fn parse_number(text: &str) -> Option<Term> {
    if text.contains('.') {
        text.parse::<f64>().ok().map(Term::Float)
    } else {
        text.parse::<i64>().ok().map(Term::Integer)
    }
}

fn number_text(term: &Term) -> String {
    match term {
        Term::Integer(number) => number.to_string(),
        Term::Float(number) => number.to_string(),
        _ => String::new(),
    }
}
