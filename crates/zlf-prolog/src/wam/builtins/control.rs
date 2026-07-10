use super::builtin_executor::BuiltinExecutor;
use super::cell::Cell;
use super::error::WamResult;
use super::execution_result::StepOutcome;
use super::executor::WamExecutor;
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;
use super::program::WamProgram;
use zlf_storage::Storage;

impl WamExecutor {
    pub(crate) fn meta_call_outcome(
        &mut self,
        call_arity: usize,
        program: &WamProgram,
        return_pc: Option<usize>,
        provider: Option<&dyn FactProvider>,
        storage: Option<&Storage>,
    ) -> WamResult<StepOutcome> {
        let closure = self.callable_from_register(0)?;
        let Some((name, closure_args)) = closure else {
            return Ok(StepOutcome::Failed);
        };
        let extras = (1..call_arity)
            .map(|register| self.registers.get(register))
            .collect::<WamResult<Vec<_>>>()?;
        let mut args = closure_args;
        args.extend(extras);
        for (register, address) in args.iter().enumerate() {
            self.registers.set(register, *address)?;
        }
        let key = PredicateKey {
            name,
            arity: args.len(),
        };
        self.dispatch_call(&key, program, return_pc, provider, storage)
    }

    pub(crate) fn dispatch_call(
        &mut self,
        key: &PredicateKey,
        program: &WamProgram,
        return_pc: Option<usize>,
        provider: Option<&dyn FactProvider>,
        storage: Option<&Storage>,
    ) -> WamResult<StepOutcome> {
        if let Some(target) = program.entry(key) {
            if let Some(return_pc) = return_pc {
                self.call_stack.push(return_pc);
                self.cut_base_stack.push(self.choice_points.len());
            }
            return Ok(StepOutcome::Jump(target));
        }
        match BuiltinExecutor::execute(self, key, storage)? {
            Some(true) => Ok(self.builtin_success_outcome(return_pc)),
            Some(false) => Ok(StepOutcome::Failed),
            None => self
                .try_provider_call(key, return_pc, provider, storage)
                .map(|outcome| {
                    outcome.unwrap_or_else(|| {
                        if program.has_entries() {
                            StepOutcome::Failed
                        } else {
                            StepOutcome::Continue
                        }
                    })
                }),
        }
    }

    fn callable_from_register(&self, register: usize) -> WamResult<Option<(String, Vec<usize>)>> {
        let address = self.machine.heap().deref(self.registers.get(register)?)?;
        match self.machine.heap().cell(address)? {
            Cell::Constant(value) => match super::constant::decode(value) {
                crate::parser::Term::Atom(name) => Ok(Some((name, Vec::new()))),
                _ => Ok(None),
            },
            Cell::Str(_) => {
                let (name, arity, first_arg) = self.machine.heap().structure_parts(address)?;
                Ok(Some((
                    name.to_string(),
                    (0..arity).map(|offset| first_arg + offset).collect(),
                )))
            }
            _ => Ok(None),
        }
    }

    fn builtin_success_outcome(&mut self, return_pc: Option<usize>) -> StepOutcome {
        if return_pc.is_some() {
            StepOutcome::Continue
        } else {
            self.return_from_call()
                .map_or(StepOutcome::Continue, StepOutcome::Jump)
        }
    }
}
