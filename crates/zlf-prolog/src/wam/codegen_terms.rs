use std::collections::HashMap;

use crate::parser::Term;

use super::codegen::WamCodegen;
use super::error::{WamError, WamResult};
use super::instruction::Instruction;

impl WamCodegen {
    pub(crate) fn put_variable(
        &mut self,
        name: &str,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        if let Some(slot) = self.permanent_slots.get(name) {
            instructions.push(Instruction::PutPermanentValue {
                slot: *slot,
                register,
            });
        } else if let Some(source) = self.var_regs.get(name) {
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

    pub(crate) fn put_structure(
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

    pub(crate) fn put_list(
        &mut self,
        items: &[Term],
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        let nested = self.precompile_nested(items, instructions)?;
        instructions.push(Instruction::PutList {
            arity: items.len(),
            register,
        });
        self.emit_set_args(items, &nested, instructions)
    }

    pub(crate) fn precompile_nested(
        &mut self,
        args: &[Term],
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<HashMap<usize, usize>> {
        let mut nested = HashMap::new();
        for (index, arg) in args.iter().enumerate() {
            if matches!(arg, Term::Compound { .. } | Term::List(_)) {
                let register = self.allocate_temp();
                self.compile_put(arg, register, instructions)?;
                nested.insert(index, register);
            }
        }
        Ok(nested)
    }

    pub(crate) fn emit_set_args(
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

    pub(crate) fn emit_set_arg(
        &mut self,
        arg: &Term,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        match arg {
            Term::Variable(name) => self.set_variable(name, instructions),
            _ => instructions.push(Instruction::SetConstant {
                value: constant_value(arg)?,
            }),
        }
        Ok(())
    }

    pub(crate) fn set_variable(&mut self, name: &str, instructions: &mut Vec<Instruction>) {
        if let Some(slot) = self.permanent_slots.get(name) {
            instructions.push(Instruction::SetPermanentValue { slot: *slot });
        } else if let Some(register) = self.var_regs.get(name) {
            instructions.push(Instruction::SetValue {
                register: *register,
            });
        } else {
            let register = self.allocate_temp();
            self.var_regs.insert(name.to_string(), register);
            instructions.push(Instruction::SetVariable { register });
        }
    }

    pub(crate) fn put_constant(
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

    pub(crate) fn get_variable(
        &mut self,
        name: &str,
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        if let Some(slot) = self.permanent_slots.get(name) {
            instructions.push(Instruction::GetPermanentValue {
                slot: *slot,
                register,
            });
        } else if let Some(source) = self.var_regs.get(name) {
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

    pub(crate) fn get_structure(
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
        self.emit_get_args(args, instructions)
    }

    pub(crate) fn get_list(
        &mut self,
        items: &[Term],
        register: usize,
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        instructions.push(Instruction::GetList {
            arity: items.len(),
            register,
        });
        self.emit_get_args(items, instructions)
    }

    fn emit_get_args(
        &mut self,
        args: &[Term],
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<()> {
        let pending = self.emit_unify_args(args, instructions)?;
        for (term, register) in pending {
            self.compile_get(&term, register, instructions)?;
        }
        Ok(())
    }

    pub(crate) fn emit_unify_args(
        &mut self,
        args: &[Term],
        instructions: &mut Vec<Instruction>,
    ) -> WamResult<Vec<(Term, usize)>> {
        let mut pending = Vec::new();
        for arg in args {
            match arg {
                Term::Variable(name) => self.unify_variable(name, instructions),
                Term::Compound { .. } | Term::List(_) => {
                    self.unify_nested(arg, instructions, &mut pending)
                }
                _ => instructions.push(Instruction::UnifyConstant {
                    value: constant_value(arg)?,
                }),
            }
        }
        Ok(pending)
    }

    pub(crate) fn unify_variable(&mut self, name: &str, instructions: &mut Vec<Instruction>) {
        if let Some(slot) = self.permanent_slots.get(name) {
            instructions.push(Instruction::UnifyPermanentValue { slot: *slot });
        } else if let Some(register) = self.var_regs.get(name) {
            instructions.push(Instruction::UnifyValue {
                register: *register,
            });
        } else {
            let register = self.allocate_temp();
            self.var_regs.insert(name.to_string(), register);
            instructions.push(Instruction::UnifyVariable { register });
        }
    }

    pub(crate) fn unify_nested(
        &mut self,
        arg: &Term,
        instructions: &mut Vec<Instruction>,
        pending: &mut Vec<(Term, usize)>,
    ) {
        let register = self.allocate_temp();
        instructions.push(Instruction::UnifyVariable { register });
        pending.push((arg.clone(), register));
    }

    pub(crate) fn get_constant(
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

    pub(crate) fn allocate_temp(&mut self) -> usize {
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
