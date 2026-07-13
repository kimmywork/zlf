use std::sync::{Arc, Mutex};

use super::error::{WamError, WamResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexAnswerLimits {
    pub candidate_limit: usize,
    pub answer_limit: usize,
}

impl Default for IndexAnswerLimits {
    fn default() -> Self {
        Self {
            candidate_limit: 10_000,
            answer_limit: 10_000,
        }
    }
}

impl IndexAnswerLimits {
    pub(super) fn validate(self) -> WamResult<()> {
        if self.answer_limit == 0 || self.candidate_limit < self.answer_limit {
            return Err(WamError::Provider(
                "index candidate limit must cover a positive answer limit".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexAnswerMetrics {
    pub calls: u64,
    pub candidates_produced: u64,
    pub answers_produced: u64,
    pub peak_materialized_answers: usize,
    pub candidate_budget_exhaustions: u64,
    pub answer_budget_exhaustions: u64,
}

#[derive(Clone, Default)]
pub(super) struct IndexAnswerState {
    metrics: Arc<Mutex<IndexAnswerMetrics>>,
}

impl IndexAnswerState {
    pub(super) fn finish<T>(
        &self,
        mut candidates: Vec<T>,
        limits: IndexAnswerLimits,
        candidate_budget_exhausted: bool,
    ) -> Vec<T> {
        let candidate_count = candidates.len();
        let answer_budget_exhausted = candidate_count > limits.answer_limit;
        candidates.truncate(limits.answer_limit);
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.calls += 1;
            metrics.candidates_produced += candidate_count as u64;
            metrics.answers_produced += candidates.len() as u64;
            metrics.peak_materialized_answers =
                metrics.peak_materialized_answers.max(candidates.len());
            metrics.candidate_budget_exhaustions += u64::from(candidate_budget_exhausted);
            metrics.answer_budget_exhaustions += u64::from(answer_budget_exhausted);
        }
        candidates
    }

    pub(super) fn snapshot(&self) -> IndexAnswerMetrics {
        self.metrics
            .lock()
            .map(|metrics| *metrics)
            .unwrap_or_default()
    }
}
