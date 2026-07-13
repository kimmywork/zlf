use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{GenerationId, IndexDocumentId, SourceRange, VectorMetric};

pub const DEFAULT_RRF_K: f64 = 60.0;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid retrieval contract: {0}")]
pub struct RetrievalContractError(pub String);

impl From<&str> for RetrievalContractError {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    Lexical,
    Vector,
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultAggregation {
    Document,
    Entity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RetrievalQuery {
    Text {
        text: String,
    },
    Vector {
        values: Vec<f32>,
        metric: VectorMetric,
    },
    SourceDocument {
        document_id: IndexDocumentId,
    },
    Prepared {
        handle: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalFilter {
    EventRange { start_micros: i64, end_micros: i64 },
    ValidAt { instant_micros: i64 },
    ValidOverlaps { start_micros: i64, end_micros: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalBudgets {
    pub candidate_k: usize,
    pub page_size: usize,
    pub max_pages: usize,
    pub max_answers: usize,
}

impl RetrievalBudgets {
    fn validate(&self, top_k: usize) -> Result<(), RetrievalContractError> {
        if self.candidate_k < top_k || self.page_size == 0 || self.max_pages == 0 {
            return Err("candidate_k must cover top_k and page/page-count must be positive".into());
        }
        if self.max_answers < top_k || self.page_size > self.candidate_k {
            return Err("answer budget must cover top_k and page_size must fit candidate_k".into());
        }
        if self.page_size.saturating_mul(self.max_pages) < self.candidate_k {
            return Err("page budget must cover candidate_k".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalRequest {
    pub query: RetrievalQuery,
    pub mode: RetrievalMode,
    pub profiles: Vec<String>,
    pub top_k: usize,
    pub budgets: RetrievalBudgets,
    pub threshold: Option<f32>,
    pub fields: Vec<String>,
    pub model_generation: Option<GenerationId>,
    pub analyzer_generation: Option<GenerationId>,
    pub temporal_filter: Option<TemporalFilter>,
    pub exclude_source: Option<IndexDocumentId>,
    pub graph_filter_goal: Option<String>,
    pub aggregation: ResultAggregation,
    pub explain: bool,
}

impl RetrievalRequest {
    pub fn validate(&self) -> Result<(), RetrievalContractError> {
        if self.top_k == 0 {
            return Err("top_k must be positive".into());
        }
        self.budgets.validate(self.top_k)?;
        if self.threshold.is_some_and(|value| !value.is_finite()) {
            return Err("threshold must be finite".into());
        }
        validate_query(&self.query)?;
        validate_temporal_filter(self.temporal_filter.as_ref())
    }
}

fn validate_query(query: &RetrievalQuery) -> Result<(), RetrievalContractError> {
    match query {
        RetrievalQuery::Text { text } if text.trim().is_empty() => {
            Err("query text must be nonempty".into())
        }
        RetrievalQuery::Vector { values, .. }
            if values.is_empty() || values.iter().any(|value| !value.is_finite()) =>
        {
            Err("query vector must be nonempty and finite".into())
        }
        RetrievalQuery::Prepared { handle } if handle.is_empty() => {
            Err("prepared handle must be nonempty".into())
        }
        _ => Ok(()),
    }
}

fn validate_temporal_filter(filter: Option<&TemporalFilter>) -> Result<(), RetrievalContractError> {
    match filter {
        Some(
            TemporalFilter::EventRange {
                start_micros,
                end_micros,
            }
            | TemporalFilter::ValidOverlaps {
                start_micros,
                end_micros,
            },
        ) if start_micros >= end_micros => {
            Err("temporal range must be nonempty and increasing".into())
        }
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrieverScore {
    pub rank: usize,
    pub score: f32,
    pub generation: GenerationId,
    pub watermark: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalHit {
    pub document_id: IndexDocumentId,
    pub source_range: Option<SourceRange>,
    pub fused_rank: usize,
    pub fused_score: f64,
    pub lexical: Option<RetrieverScore>,
    pub vector: Option<RetrieverScore>,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankedRetrieverHit {
    pub document_id: IndexDocumentId,
    pub score: f32,
    pub generation: GenerationId,
    pub watermark: u64,
    pub source_range: Option<SourceRange>,
}

#[derive(Default)]
struct FusionAccumulator {
    fused_score: f64,
    lexical: Option<RetrieverScore>,
    vector: Option<RetrieverScore>,
    source_range: Option<SourceRange>,
}

pub fn reciprocal_rank_fusion(
    lexical: &[RankedRetrieverHit],
    vector: &[RankedRetrieverHit],
    top_k: usize,
    rrf_k: f64,
) -> Result<Vec<RetrievalHit>, RetrievalContractError> {
    if top_k == 0 || !rrf_k.is_finite() || rrf_k <= 0.0 {
        return Err("top_k and finite positive rrf_k are required".into());
    }
    let mut fused = BTreeMap::new();
    add_ranks(&mut fused, lexical, rrf_k, true)?;
    add_ranks(&mut fused, vector, rrf_k, false)?;
    let mut hits = fused
        .into_iter()
        .map(|(document_id, value)| RetrievalHit {
            document_id,
            source_range: value.source_range,
            fused_rank: 0,
            fused_score: value.fused_score,
            lexical: value.lexical,
            vector: value.vector,
            explanation: None,
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .fused_score
            .total_cmp(&left.fused_score)
            .then_with(|| left.document_id.cmp(&right.document_id))
    });
    hits.truncate(top_k);
    for (index, hit) in hits.iter_mut().enumerate() {
        hit.fused_rank = index + 1;
    }
    Ok(hits)
}

fn add_ranks(
    fused: &mut BTreeMap<IndexDocumentId, FusionAccumulator>,
    hits: &[RankedRetrieverHit],
    rrf_k: f64,
    lexical: bool,
) -> Result<(), RetrievalContractError> {
    let mut seen = BTreeSet::new();
    for (index, hit) in hits.iter().enumerate() {
        if !hit.score.is_finite() {
            return Err("retriever scores must be finite".into());
        }
        if !seen.insert(hit.document_id.clone()) {
            continue;
        }
        let rank = index + 1;
        let score = RetrieverScore {
            rank,
            score: hit.score,
            generation: hit.generation.clone(),
            watermark: hit.watermark,
        };
        let target = fused.entry(hit.document_id.clone()).or_default();
        target.fused_score += 1.0 / (rrf_k + rank as f64);
        target.source_range = target.source_range.or(hit.source_range);
        if lexical {
            target.lexical = Some(score);
        } else {
            target.vector = Some(score);
        }
    }
    Ok(())
}
