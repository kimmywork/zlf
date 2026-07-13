use serde::{Deserialize, Serialize};

use crate::RetrievalContractError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexPageRequest {
    pub offset: usize,
    pub page_size: usize,
    pub candidate_limit: usize,
}

impl IndexPageRequest {
    pub fn validate(self) -> Result<(), RetrievalContractError> {
        if self.page_size == 0 || self.candidate_limit == 0 || self.offset >= self.candidate_limit {
            return Err("page requires positive limits and offset below candidate limit".into());
        }
        Ok(())
    }

    pub fn probe_limit(self) -> usize {
        self.offset
            .saturating_add(self.page_size)
            .saturating_add(1)
            .min(self.candidate_limit)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexPage<T> {
    pub items: Vec<T>,
    pub next_offset: Option<usize>,
    pub candidates_scanned: u64,
    pub candidate_budget_exhausted: bool,
}

pub fn ranked_page<T>(
    mut prefix: Vec<T>,
    request: IndexPageRequest,
) -> Result<IndexPage<T>, RetrievalContractError> {
    request.validate()?;
    let available = prefix.len();
    let end = request
        .offset
        .saturating_add(request.page_size)
        .min(available)
        .min(request.candidate_limit);
    let has_more = available > end;
    let exhausted = end == request.candidate_limit && available >= request.candidate_limit;
    prefix.truncate(end);
    let items = prefix.drain(request.offset.min(prefix.len())..).collect();
    Ok(IndexPage {
        items,
        next_offset: (has_more && end < request.candidate_limit).then_some(end),
        candidates_scanned: available as u64,
        candidate_budget_exhausted: exhausted,
    })
}
