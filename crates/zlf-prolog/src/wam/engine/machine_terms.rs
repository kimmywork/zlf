use std::collections::HashMap;

use crate::parser::Term;

use super::constant;
use super::error::WamResult;
use super::machine::M0Machine;

impl M0Machine {
    pub(crate) fn put_term(&mut self, term: &Term) -> WamResult<usize> {
        self.put_term_with_variables(term, &mut HashMap::new())
    }

    pub(crate) fn put_terms_shared(&mut self, terms: &[Term]) -> WamResult<Vec<usize>> {
        let mut variables = HashMap::new();
        terms
            .iter()
            .map(|term| self.put_term_with_variables(term, &mut variables))
            .collect()
    }

    fn put_term_with_variables(
        &mut self,
        term: &Term,
        variables: &mut HashMap<String, usize>,
    ) -> WamResult<usize> {
        match term {
            Term::Variable(name) if name != "_" => {
                if let Some(address) = variables.get(name) {
                    Ok(*address)
                } else {
                    let address = self.set_variable();
                    variables.insert(name.clone(), address);
                    Ok(address)
                }
            }
            Term::Variable(_) => Ok(self.set_variable()),
            Term::Atom(_) | Term::String(_) | Term::Integer(_) | Term::Float(_) => {
                Ok(self.put_constant(constant::encode(term)?))
            }
            Term::Compound { name, args } => self.put_compound_term(name, args, variables),
            Term::List(items) => {
                let canonical = canonical_list(items);
                self.put_term_with_variables(&canonical, variables)
            }
            Term::Object(entries) => {
                let object = object_term(entries);
                self.put_term_with_variables(&object, variables)
            }
        }
    }

    fn put_compound_term(
        &mut self,
        name: &str,
        args: &[Term],
        variables: &mut HashMap<String, usize>,
    ) -> WamResult<usize> {
        let addresses = args
            .iter()
            .map(|arg| self.put_term_with_variables(arg, variables))
            .collect::<WamResult<Vec<_>>>()?;
        let structure = self.put_structure(name, args.len());
        for address in addresses {
            self.set_value(address)?;
        }
        Ok(structure)
    }
}

fn canonical_list(items: &[Term]) -> Term {
    items
        .iter()
        .rev()
        .fold(Term::Atom("[]".to_string()), |tail, head| Term::Compound {
            name: ".".to_string(),
            args: vec![head.clone(), tail],
        })
}

fn object_term(entries: &[(String, Term)]) -> Term {
    Term::Compound {
        name: "object".to_string(),
        args: entries
            .iter()
            .map(|(key, value)| Term::Compound {
                name: "pair".to_string(),
                args: vec![Term::Atom(key.clone()), value.clone()],
            })
            .collect(),
    }
}
