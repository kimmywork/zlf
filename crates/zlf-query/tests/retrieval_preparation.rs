use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use zlf_core::{EntityRef, Node};
use zlf_index::{
    EmbeddingModelProfile, GenerationId, ResultAggregation, RetrievalBudgets, RetrievalMode,
    RetrievalQuery, RetrievalRequest,
};
use zlf_query::{
    BatchEmbeddingProvider, EmbeddingProviderFailure, RetrievalPreparationError, ZlfDatabase,
};

#[tokio::test]
async fn async_preparation_embeds_once_and_registry_lookup_never_calls_provider() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
    db.add_node(Node::with_id(
        "doc".into(),
        vec!["record".into()],
        Default::default(),
    ))
    .unwrap();
    let provider = CountingProvider::default();
    let handle = db
        .prepare_retrieval(&provider, request(RetrievalMode::Hybrid))
        .await
        .unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let prepared = db.prepared_retrieval(&handle).unwrap();
    assert_eq!(prepared.handle, handle);
    assert_eq!(prepared.query_vector.as_ref().unwrap().len(), 1024);
    assert_eq!(prepared.snapshot.model_id, "bge_m3_dense_v1");
    assert!(!prepared.snapshot.lexical_generation.0.is_empty());
    assert!(!prepared.snapshot.vector_generation.0.is_empty());
    assert!(!prepared.snapshot.temporal_generation.0.is_empty());

    db.prepared_retrieval(&handle).unwrap();
    db.query_prolog("? node(Node).").unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert!(db.release_prepared_retrieval(&handle).unwrap());
    assert!(matches!(
        db.prepared_retrieval(&handle),
        Err(RetrievalPreparationError::UnknownHandle(_))
    ));
}

#[tokio::test]
async fn lexical_and_explicit_vectors_require_no_remote_embedding() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
    let provider = CountingProvider::default();
    let lexical = db
        .prepare_retrieval(&provider, request(RetrievalMode::Lexical))
        .await
        .unwrap();
    assert!(db
        .prepared_retrieval(&lexical)
        .unwrap()
        .query_vector
        .is_none());

    let mut vector = request(RetrievalMode::Vector);
    let mut values = vec![0.0; 1024];
    values[0] = 1.0;
    vector.query = RetrievalQuery::Vector {
        values,
        metric: zlf_index::VectorMetric::Cosine,
    };
    let vector = db.prepare_retrieval(&provider, vector).await.unwrap();
    assert_eq!(
        db.prepared_retrieval(&vector)
            .unwrap()
            .query_vector
            .unwrap()
            .len(),
        1024
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn incompatible_generation_and_provider_failure_are_typed_before_wam() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
    let provider = CountingProvider::default();
    let mut incompatible = request(RetrievalMode::Hybrid);
    incompatible.model_generation = Some(GenerationId("not-active".into()));
    assert!(matches!(
        db.prepare_retrieval(&provider, incompatible).await,
        Err(RetrievalPreparationError::GenerationMismatch {
            target: "model",
            ..
        })
    ));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);

    let mut watermark = request(RetrievalMode::Lexical);
    watermark.minimum_watermarks.insert("bm25".into(), 999);
    watermark.wait_timeout_ms = 10;
    assert!(matches!(
        db.prepare_retrieval(&provider, watermark).await,
        Err(RetrievalPreparationError::WatermarkTimeout {
            target,
            minimum: 999,
            ..
        }) if target == "bm25"
    ));

    let failure = FailingProvider;
    assert!(matches!(
        db.prepare_retrieval(&failure, request(RetrievalMode::Hybrid))
            .await,
        Err(RetrievalPreparationError::Embedding(_))
    ));
}

fn request(mode: RetrievalMode) -> RetrievalRequest {
    RetrievalRequest {
        query: RetrievalQuery::Text {
            text: "knowledge".into(),
        },
        mode,
        profiles: Vec::new(),
        top_k: 10,
        budgets: RetrievalBudgets {
            candidate_k: 100,
            page_size: 20,
            max_pages: 5,
            max_answers: 10,
        },
        threshold: None,
        fields: Vec::new(),
        model_generation: None,
        analyzer_generation: None,
        temporal_filter: None,
        exclude_source: Some(zlf_index::IndexDocumentId::new(
            EntityRef::Node("source".into()),
            "body",
            "0",
        )),
        graph_filter_goal: None,
        minimum_watermarks: std::collections::BTreeMap::new(),
        wait_timeout_ms: 1_000,
        aggregation: ResultAggregation::Document,
        explain: false,
    }
}

#[derive(Default)]
struct CountingProvider {
    calls: AtomicUsize,
}

#[async_trait]
impl BatchEmbeddingProvider for CountingProvider {
    async fn embed_query(
        &self,
        _model: &EmbeddingModelProfile,
        _text: &str,
    ) -> Result<Vec<f32>, EmbeddingProviderFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![1.0; 1024])
    }

    async fn embed_documents(
        &self,
        _model: &EmbeddingModelProfile,
        inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingProviderFailure> {
        Ok(vec![vec![1.0; 1024]; inputs.len()])
    }
}

struct FailingProvider;

#[async_trait]
impl BatchEmbeddingProvider for FailingProvider {
    async fn embed_query(
        &self,
        _model: &EmbeddingModelProfile,
        _text: &str,
    ) -> Result<Vec<f32>, EmbeddingProviderFailure> {
        Err(EmbeddingProviderFailure {
            class: "provider_unavailable".into(),
            retryable: true,
        })
    }

    async fn embed_documents(
        &self,
        _model: &EmbeddingModelProfile,
        _inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingProviderFailure> {
        unreachable!()
    }
}
