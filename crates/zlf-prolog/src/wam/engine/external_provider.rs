use crate::parser::Term;

use super::choice_point::ChoicePointFrame;
use super::error::WamResult;
use super::execution_result::StepOutcome;
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;
use super::storage_provider::StorageFactProvider;
use super::WamExecutor;
use zlf_storage::Storage;

impl WamExecutor {
    pub(crate) fn try_provider_call(
        &mut self,
        key: &PredicateKey,
        return_pc: Option<usize>,
        provider: Option<&dyn FactProvider>,
        storage: Option<&Storage>,
    ) -> WamResult<Option<StepOutcome>> {
        let Some(mut answers) = self.provider_answers(key, provider, storage)? else {
            return Ok(None);
        };
        if answers.is_empty() {
            return Ok(Some(StepOutcome::Failed));
        }
        let first = answers.remove(0);
        self.install_external_choices(key, return_pc, answers);
        if !self.apply_external_answer(key, &first)? {
            return self.backtrack_target().map(|target| {
                target.map_or(Some(StepOutcome::Failed), |pc| Some(StepOutcome::Jump(pc)))
            });
        }
        Ok(Some(self.external_success(return_pc)))
    }

    fn provider_answers(
        &self,
        key: &PredicateKey,
        provider: Option<&dyn FactProvider>,
        storage: Option<&Storage>,
    ) -> WamResult<Option<Vec<Vec<Term>>>> {
        let goal = Term::Compound {
            name: key.name.clone(),
            args: (0..key.arity)
                .map(|register| self.register_term(register))
                .collect::<WamResult<Vec<_>>>()?,
        };
        let storage_provider = storage.map(StorageFactProvider::new);
        let provider = provider.or_else(|| {
            storage_provider
                .as_ref()
                .map(|provider| provider as &dyn FactProvider)
        });
        let Some(provider) = provider else {
            return Ok(None);
        };
        Ok(Some(
            provider
                .facts_for_goal(&goal)?
                .into_iter()
                .filter_map(|fact| fact_args(fact, key))
                .collect(),
        ))
    }

    fn install_external_choices(
        &mut self,
        key: &PredicateKey,
        return_pc: Option<usize>,
        answers: Vec<Vec<Term>>,
    ) {
        if answers.is_empty() {
            return;
        }
        let continuation = return_pc.or_else(|| self.call_stack.last().copied());
        self.choice_points.push(ChoicePointFrame::capture_external(
            &self.machine,
            &self.registers,
            &self.environments,
            &self.call_stack,
            &self.cut_base_stack,
            continuation.unwrap_or(0),
            key.clone(),
            answers,
            self.proof.checkpoint(),
            return_pc.is_none(),
        ));
    }

    pub(crate) fn backtrack_target(&mut self) -> WamResult<Option<usize>> {
        loop {
            let Some(frame) = self.choice_points.last() else {
                return Ok(None);
            };
            if !frame.is_external() {
                return Ok(Some(frame.next_alternative()));
            }
            let frame = self.choice_points.pop().expect("choice frame exists");
            if let Some(target) = self.retry_external_frame(frame)? {
                return Ok(Some(target));
            }
        }
    }

    fn retry_external_frame(&mut self, mut frame: ChoicePointFrame) -> WamResult<Option<usize>> {
        frame.restore(
            &mut self.machine,
            &mut self.registers,
            &mut self.environments,
        )?;
        self.call_stack = frame.call_stack();
        self.cut_base_stack = frame.cut_base_stack();
        self.proof.restore(frame.proof_checkpoint());
        let Some(answer) = frame.next_external_answer() else {
            return Ok(None);
        };
        let continuation = frame.continuation().unwrap_or_default();
        let predicate = frame
            .external_predicate()
            .expect("external choice has a predicate");
        let tail_call = frame.external_tail_call();
        if frame.has_external_answers() {
            self.choice_points.push(frame);
        }
        if !self.apply_external_answer(&predicate, &answer)? {
            return Ok(None);
        }
        if tail_call {
            self.return_from_call();
        }
        Ok(Some(continuation))
    }

    fn apply_external_answer(&mut self, key: &PredicateKey, answer: &[Term]) -> WamResult<bool> {
        for (register, term) in answer.iter().enumerate() {
            let address = self.machine.put_term(term)?;
            if !self.machine.unify(self.registers.get(register)?, address)? {
                return Ok(false);
            }
        }
        if self.proof.is_enabled() {
            let fact = if key.arity == 0 {
                Term::Atom(key.name.clone())
            } else {
                Term::Compound {
                    name: key.name.clone(),
                    args: answer.to_vec(),
                }
            };
            if let Some(clause) = super::proof::fact_clause(&fact) {
                self.proof.record_leaf(clause, answer.to_vec());
            }
        }
        Ok(true)
    }

    fn external_success(&mut self, return_pc: Option<usize>) -> StepOutcome {
        if return_pc.is_some() {
            StepOutcome::Continue
        } else {
            self.return_from_call()
                .map_or(StepOutcome::Continue, StepOutcome::Jump)
        }
    }
}

fn fact_args(fact: Term, key: &PredicateKey) -> Option<Vec<Term>> {
    match fact {
        Term::Compound { name, args } if name == key.name && args.len() == key.arity => Some(args),
        Term::Atom(name) if name == key.name && key.arity == 0 => Some(Vec::new()),
        _ => None,
    }
}
