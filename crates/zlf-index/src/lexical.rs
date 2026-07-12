use jieba_rs::Jieba;
use serde::{Deserialize, Serialize};

use crate::{GenerationId, IndexDocumentId};

pub const UNICODE_JIEBA_ANALYZER_ID: &str = "unicode_jieba_v1";
pub const UNICODE_JIEBA_ANALYZER_VERSION: u32 = 1;
pub const TANTIVY_BM25_K1: f32 = 1.2;
pub const TANTIVY_BM25_B: f32 = 0.75;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bm25Config {
    pub k1: f32,
    pub b: f32,
    pub top_k: usize,
    pub candidate_limit: usize,
}

impl Default for Bm25Config {
    fn default() -> Self {
        Self {
            k1: TANTIVY_BM25_K1,
            b: TANTIVY_BM25_B,
            top_k: 10,
            candidate_limit: 10_000,
        }
    }
}

impl Bm25Config {
    pub fn validate(self) -> Result<(), String> {
        if !self.k1.is_finite()
            || self.k1 <= 0.0
            || !self.b.is_finite()
            || !(0.0..=1.0).contains(&self.b)
            || self.top_k == 0
            || self.candidate_limit < self.top_k
        {
            return Err("invalid BM25 configuration".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalQuery {
    pub text: String,
    pub top_k: usize,
    pub fields: Vec<String>,
    pub generation: Option<GenerationId>,
    pub explain: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bm25Explanation {
    pub terms: Vec<TermScoreExplanation>,
    pub document_length: u64,
    pub average_document_length: f32,
    pub field_weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermScoreExplanation {
    pub term: String,
    pub term_frequency: u64,
    pub document_frequency: u64,
    pub inverse_document_frequency: f32,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LexicalHit {
    pub document_id: IndexDocumentId,
    pub score: f32,
    pub rank: usize,
    pub generation: GenerationId,
    pub explanation: Option<Bm25Explanation>,
}

pub struct UnicodeJiebaAnalyzer {
    jieba: Jieba,
}

impl Default for UnicodeJiebaAnalyzer {
    fn default() -> Self {
        Self {
            jieba: Jieba::new(),
        }
    }
}

impl UnicodeJiebaAnalyzer {
    pub fn id(&self) -> &'static str {
        UNICODE_JIEBA_ANALYZER_ID
    }

    pub fn analyze(&self, text: &str) -> Vec<String> {
        self.jieba
            .cut(text, false)
            .iter()
            .map(|word| word.word.trim().to_lowercase())
            .filter(|word| !word.is_empty())
            .collect()
    }
}

pub fn bm25_term_score(
    term_frequency: u64,
    document_frequency: u64,
    document_count: u64,
    document_length: u64,
    average_document_length: f64,
    k1: f64,
    b: f64,
) -> f64 {
    if term_frequency == 0
        || document_frequency == 0
        || document_count == 0
        || average_document_length <= 0.0
    {
        return 0.0;
    }
    let tf = term_frequency as f64;
    let df = document_frequency as f64;
    let count = document_count as f64;
    let idf = (1.0 + (count - df + 0.5) / (df + 0.5)).ln();
    let normalization = 1.0 - b + b * document_length as f64 / average_document_length;
    idf * (tf * (k1 + 1.0)) / (tf + k1 * normalization)
}
