use super::predicate::PredicateKey;

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionResult {
    pub success: bool,
    pub last_call: Option<PredicateKey>,
}

pub(crate) enum StepOutcome {
    Continue,
    Jump(usize),
    Failed,
}
