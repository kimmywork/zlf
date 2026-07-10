use super::builtin_arithmetic::eval_arithmetic;
use super::error::WamResult;
use super::executor::WamExecutor;
use super::predicate::PredicateKey;
use zlf_storage::Storage;

#[derive(Debug, Default, Clone, Copy)]
pub struct BuiltinExecutor;

impl BuiltinExecutor {
    pub(crate) fn execute(
        wam: &mut WamExecutor,
        key: &PredicateKey,
        storage: Option<&Storage>,
    ) -> WamResult<Option<bool>> {
        let result = if let Some(result) = wam.execute_term_builtin(&key.name, key.arity)? {
            Some(result)
        } else if let Some(result) = wam.execute_conversion_builtin(&key.name, key.arity)? {
            Some(result)
        } else if let Some(result) = wam.execute_list_builtin(&key.name, key.arity)? {
            Some(result)
        } else if let Some(result) = wam.execute_dynamic_builtin(&key.name, key.arity, storage)? {
            Some(result)
        } else {
            execute_arithmetic_or_truth(wam, key)?
        };
        if result == Some(true) {
            wam.record_builtin_proof(key)?;
        }
        Ok(result)
    }
}

fn execute_arithmetic_or_truth(
    wam: &mut WamExecutor,
    key: &PredicateKey,
) -> WamResult<Option<bool>> {
    let result = match (key.name.as_str(), key.arity) {
        ("true", 0) => true,
        ("fail", 0) | ("false", 0) => false,
        ("is", 2) => wam.eval_is_registers()?,
        ("=:=", 2) => wam.eval_arith_cmp(|left, right| (left - right).abs() < f64::EPSILON)?,
        ("=\\=", 2) => wam.eval_arith_cmp(|left, right| (left - right).abs() >= f64::EPSILON)?,
        ("<", 2) => wam.eval_arith_cmp(|left, right| left < right)?,
        ("=<", 2) => wam.eval_arith_cmp(|left, right| left <= right)?,
        (">", 2) => wam.eval_arith_cmp(|left, right| left > right)?,
        (">=", 2) => wam.eval_arith_cmp(|left, right| left >= right)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

impl WamExecutor {
    fn record_builtin_proof(&mut self, key: &PredicateKey) -> WamResult<()> {
        if self.proof.is_enabled() {
            let substitutions = (0..key.arity)
                .map(|register| self.register_term(register))
                .collect::<WamResult<Vec<_>>>()?;
            self.proof
                .record_leaf(super::proof::builtin_clause(key), substitutions);
        }
        Ok(())
    }

    fn eval_is_registers(&mut self) -> WamResult<bool> {
        let rhs = self.register_term(1)?;
        let term = eval_arithmetic(&rhs)?.into_term();
        let value_addr = self.machine.put_constant(super::constant::encode(&term)?);
        let target = self.registers.get(0)?;
        self.machine.unify(target, value_addr)
    }

    fn eval_arith_cmp(&self, cmp: fn(f64, f64) -> bool) -> WamResult<bool> {
        let left = eval_arithmetic(&self.register_term(0)?)?.as_f64();
        let right = eval_arithmetic(&self.register_term(1)?)?.as_f64();
        Ok(cmp(left, right))
    }
}
