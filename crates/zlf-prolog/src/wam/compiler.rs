use std::collections::HashMap;

use crate::parser::Term;

use super::error::WamResult;
use super::machine::M0Machine;

#[derive(Debug, Default)]
pub struct M0Compiler {
    variables: HashMap<String, usize>,
}

impl M0Compiler {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn variables(&self) -> &HashMap<String, usize> {
        &self.variables
    }

    pub fn compile_term(&mut self, machine: &mut M0Machine, term: &Term) -> WamResult<usize> {
        match term {
            Term::Variable(name) => self.compile_variable(machine, name),
            Term::Atom(value) | Term::String(value) => Ok(machine.put_constant(value)),
            Term::Number(value) => Ok(machine.put_constant(value.to_string())),
            Term::Compound { name, args } => self.compile_compound(machine, name, args),
            Term::List(items) => self.compile_list(machine, items),
            Term::Object(entries) => self.compile_object(machine, entries),
        }
    }

    fn compile_variable(&mut self, machine: &mut M0Machine, name: &str) -> WamResult<usize> {
        if let Some(addr) = self.variables.get(name) {
            return Ok(*addr);
        }
        let addr = machine.set_variable();
        self.variables.insert(name.to_string(), addr);
        Ok(addr)
    }

    fn compile_compound(
        &mut self,
        machine: &mut M0Machine,
        name: &str,
        args: &[Term],
    ) -> WamResult<usize> {
        let arg_addrs = self.compile_args(machine, args)?;
        let root = machine.put_structure(name, arg_addrs.len());
        for addr in arg_addrs {
            machine.set_value(addr)?;
        }
        Ok(root)
    }

    fn compile_list(&mut self, machine: &mut M0Machine, items: &[Term]) -> WamResult<usize> {
        let arg_addrs = self.compile_args(machine, items)?;
        let root = machine.put_structure("list", arg_addrs.len());
        for addr in arg_addrs {
            machine.set_value(addr)?;
        }
        Ok(root)
    }

    fn compile_object(
        &mut self,
        machine: &mut M0Machine,
        entries: &[(String, Term)],
    ) -> WamResult<usize> {
        let pairs = entries
            .iter()
            .map(|(key, value)| Term::Compound {
                name: "pair".to_string(),
                args: vec![Term::Atom(key.clone()), value.clone()],
            })
            .collect::<Vec<_>>();
        self.compile_compound(machine, "object", &pairs)
    }

    fn compile_args(&mut self, machine: &mut M0Machine, args: &[Term]) -> WamResult<Vec<usize>> {
        args.iter()
            .map(|arg| self.compile_term(machine, arg))
            .collect()
    }
}
