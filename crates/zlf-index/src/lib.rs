pub mod bm25;
mod bm25_support;
pub mod chunking;
pub mod generation;
pub mod identity;
pub mod lexical;
pub mod manifest;
pub mod metrics;
pub mod model;
pub mod profile;
pub mod retrieval;
pub mod temporal;
pub mod vector;

pub use bm25::{BM25DocumentHit, BM25Index};
pub use chunking::{
    accept_explicit_chunks, chunk_text, content_fingerprint, ExplicitChunk, IndexChunk,
};
pub use generation::{
    GenerationId, GenerationMetadata, GenerationState, IndexStatus, IndexWaitResult,
    GENERATION_SCHEMA_VERSION,
};
pub use identity::{
    ContentFingerprint, IndexDocument, IndexDocumentId, SourceRange, INDEX_DOCUMENT_SCHEMA_VERSION,
};
pub use lexical::{
    bm25_term_score, Bm25Config, Bm25Explanation, LexicalHit, LexicalQuery, TermScoreExplanation,
    UnicodeJiebaAnalyzer, UNICODE_JIEBA_ANALYZER_ID,
};
pub use manifest::{reconcile_manifest, DocumentChanges, DocumentManifest};
pub use metrics::{IndexInventory, IndexJobMetrics, IndexMetricsSnapshot};
pub use model::{
    bge_m3_dense_v1, EmbeddingCapabilities, EmbeddingModelProfile, VectorMetric,
    EMBEDDING_MODEL_PROFILE_SCHEMA_VERSION,
};
pub use profile::{
    Bm25FieldOptions, ChunkingProfile, EntityMatcher, FieldIndexOptions, IndexProfileArtifact,
    TemporalRole, VectorFieldOptions, INDEX_PROFILE_SCHEMA_VERSION,
};
pub use retrieval::{
    RetrievalHit, RetrievalMode, RetrievalQuery, RetrievalRequest, RetrieverScore,
};
pub use temporal::{TemporalEntry, TemporalIndex};
pub use vector::{VectorEntry, VectorIndex};
