use serde::{Deserialize, Serialize};

use crate::{GenerationId, IndexDocumentId, VectorMetric};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    Lexical,
    Vector,
    Hybrid,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalRequest {
    pub query: RetrievalQuery,
    pub mode: RetrievalMode,
    pub profiles: Vec<String>,
    pub top_k: usize,
    pub candidate_k: usize,
    pub threshold: Option<f32>,
    pub fields: Vec<String>,
    pub exclude_source: bool,
    pub explain: bool,
}

impl RetrievalRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.top_k == 0 || self.candidate_k < self.top_k {
            return Err("candidate_k must be at least a positive top_k".into());
        }
        if self.threshold.is_some_and(|value| !value.is_finite()) {
            return Err("threshold must be finite".into());
        }
        if let RetrievalQuery::Vector { values, .. } = &self.query {
            if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
                return Err("query vector must be nonempty and finite".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrieverScore {
    pub rank: usize,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalHit {
    pub document_id: IndexDocumentId,
    pub fused_rank: usize,
    pub fused_score: f32,
    pub lexical: Option<RetrieverScore>,
    pub vector: Option<RetrieverScore>,
    pub generation: GenerationId,
    pub watermark: u64,
    pub explanation: Option<String>,
}
