use std::collections::HashMap;

use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::instruction::Instruction;
use super::predicate::{compound_args, predicate_key, PredicateKey};
use super::program::WamProgram;

#[derive(Debug, Default)]
pub struct WamCodegen {
    pub(crate) var_regs: HashMap<String, usize>,
    pub(crate) next_temp: usize,
}

impl WamCodegen {
    pub fn compile_query_goal(goal: &Term) -> WamResult<WamProgram> {
        let key = predicate_key(goal).ok_or(WamError::ExpectedFunctor(0))?;
        let args = compound_args(goal).ok_or(WamError::ExpectedFunctor(0))?;
        let mut codegen = Self::with_temp_start(args.len());
        let mut instructions = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            codegen.compile_put(arg, index, &mut instructions)?;
        }
        instructions.push(Instruction::Call(key));
        Ok(WamProgram::new(instructions))
    }

    pub fn compile_fact_head(fact: &Term) -> WamResult<WamProgram> {
        let args = compound_args(fact).ok_or(WamError::ExpectedFunctor(0))?;
        Self::compile_fact_head_with_temp_start(fact, args.len())
    }

    pub(crate) fn compile_fact_head_with_temp_start(
        fact: &Term,
        temp_start: usize,
    ) -> WamResult<WamProgram> {
        let args = compound_args(fact).ok_or(WamError::ExpectedFunctor(0))?;
        let mut codegen = Self::with_temp_start(temp_start.max(args.len()));
        let mut instructions = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            codegen.compile_get(arg, index, &mut instructions)?;
        }
        instructions.push(Instruction::Proceed);
        Ok(WamProgram::new(instructions))
    }

    pub fn predicate_key(term: &Term) -> WamResult<PredicateKey> {
        predicate_key(term).ok_or(WamError::ExpectedFunctor(0))
    }

    pub(crate) fn with_temp_start(next_temp: usize) -> Self {
        Self {
            var_regs: HashMap::new(),
            next_temp,
        }
    }

    pub(crate) fn compile_put(
        &mut self,
        term: &Term,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        match term {
            Term::Variable(name) => self.put_variable(name, register, instructions),
            Term::Compound { name, args } => self.put_structure(name, args, register, instructions),
            _ => self.put_constant(term, register, instructions),
        }
    }

    pub(crate) fn compile_get(
        &mut self,
        term: &Term,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        match term {
            Term::Variable(name) => self.get_variable(name, register, instructions),
            Term::Compound { name, args } => self.get_structure(name, args, register, instructions),
            _ => self.get_constant(term, register, instructions),
        }
    }

    fn put_variable(
        &mut self,
        name: &str,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        if let Some(source) = self.var_regs.get(name) {
            instructions.push(Instruction::PutValue {
                source: *source,
                target: register,
            });
        } else {
            self.var_regs.insert(name.to_string(), register);
            instructions.push(Instruction::PutVariable { register });
        }
        Ok(())
    }

    fn put_structure(
        &mut self,
        name: &str,
        args: &[Term],
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        let nested = self.precompile_nested(args, instructions)?;
        instructions.push(Instruction::PutStructure {
            name: name.to_string(),
            arity: args.len(),
            register,
        });
        self.emit_set_args(args, &nested, instructions)
    }

    fn precompile_nested(
        &mut self,
        args: &[Term],
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<HashMap<usize, usize>> {
        let mut nested = HashMap::new();
        for (index, arg) in args.iter().enumerate() {
            if matches!(arg, Term::Compound { .. }) {
                let register = self.allocate_temp();
                self.compile_put(arg, register, instructions)?;
                nested.insert(index, register);
            }
        }
        Ok(nested)
    }

    fn emit_set_args(
        &mut self,
        args: &[Term],
        nested: &HashMap<usize, usize>,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        for (index, arg) in args.iter().enumerate() {
            if let Some(register) = nested.get(&index) {
                instructions.push(Instruction::SetValue {
                    register: *register,
                });
            } else {
                self.emit_set_arg(arg, instructions)?;
            }
        }
        Ok(())
    }

    fn emit_set_arg(&mut self, arg: &Term, instructions: &mut Vec<Instruction>) -> WamResult<()> {
        match arg {
            Term::Variable(name) => self.set_variable(name, instructions),
            _ => instructions.push(Instruction::SetConstant {
                value: constant_value(arg)?,
            }),
        }
        Ok(())
    }

    fn set_variable(&mut self, name: &str, instructions: &mut Vec<Instruction>) {
        if let Some(register) = self.var_regs.get(name) {
            instructions.push(Instruction::SetValue {
                register: *register,
            });
        } else {
            let register = self.allocate_temp();
            self.var_regs.insert(name.to_string(), register);
            instructions.push(Instruction::SetVariable { register });
        }
    }

    fn put_constant(
        &mut self,
        term: &Term,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        instructions.push(Instruction::PutConstant {
            value: constant_value(term)?,
            register,
        });
        Ok(())
    }

    fn get_variable(
        &mut self,
        name: &str,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        if let Some(source) = self.var_regs.get(name) {
            instructions.push(Instruction::GetValue {
                left: *source,
                right: register,
            });
        } else {
            let target = self.allocate_temp();
            self.var_regs.insert(name.to_string(), target);
            instructions.push(Instruction::PutValue {
                source: register,
                target,
            });
        }
        Ok(())
    }

    fn get_structure(
        &mut self,
        name: &str,
        args: &[Term],
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        instructions.push(Instruction::GetStructure {
            name: name.to_string(),
            arity: args.len(),
            register,
        });
        let pending = self.emit_unify_args(args, instructions)?;
        for (term, register) in pending {
            self.compile_get(&term, register, instructions)?;
        }
        Ok(())
    }

    fn emit_unify_args(
        &mut self,
        args: &[Term],
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<Vec<(Term, usize)>> {
        let mut pending = Vec::new();
        for arg in args {
            match arg {
                Term::Variable(name) => self.unify_variable(name, instructions),
                Term::Compound { .. } => self.unify_nested(arg, instructions, &mut pending),
                _ => instructions.push(Instruction::UnifyConstant {
                    value: constant_value(arg)?,
                }),
            }
        }
        Ok(pending)
    }

    fn unify_variable(&mut self, name: &str, instructions: &mut Vec<Instruction>) {
        if let Some(register) = self.var_regs.get(name) {
            instructions.push(Instruction::UnifyValue {
                register: *register,
            });
        } else {
            let register = self.allocate_temp();
            self.var_regs.insert(name.to_string(), register);
            instructions.push(Instruction::UnifyVariable { register });
        }
    }

    fn unify_nested(
        &mut self,
        arg: &Term,
        instructions: &mut Vec<Instruction>,
        pending: &mut Vec<(Term, usize)>,
    ) {
        let register = self.allocate_temp();
        instructions.push(Instruction::UnifyVariable { register });
        pending.push((arg.clone(), register));
    }

    fn get_constant(
        &mut self,
        term: &Term,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        instructions.push(Instruction::GetConstant {
            value: constant_value(term)?,
            register,
        });
        Ok(())
    }

    fn allocate_temp(&mut self) -> usize {
        let register = self.next_temp;
        self.next_temp += 1;
        register
    }
}

fn constant_value(term: &Term) -> WamResult<String> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value.clone()),
        Term::Number(value) => Ok(value.to_string()),
        _ => Err(WamError::UnsupportedTerm("non-constant")),
    }
}
