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
mod vector_contract;
mod vector_exact;

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
    UnicodeJiebaAnalyzer, TANTIVY_BM25_B, TANTIVY_BM25_K1, UNICODE_JIEBA_ANALYZER_ID,
    UNICODE_JIEBA_ANALYZER_VERSION,
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
pub use vector_contract::{
    EmbeddingJob, EmbeddingJobState, VectorHit, VectorKey, VectorQuery, VectorRecord,
    EMBEDDING_JOB_SCHEMA_VERSION, VECTOR_RECORD_SCHEMA_VERSION,
};
pub use vector_exact::ExactVectorStore;
