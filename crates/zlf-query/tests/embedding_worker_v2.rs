use std::cell::RefCell;
use std::collections::HashMap;

use chrono::{Duration, Utc};
use zlf_core::{EntityRef, Node, Value};
use zlf_index::{
    bge_m3_dense_v1, content_fingerprint, DocumentManifest, EmbeddingJob, EmbeddingJobState,
    ExactVectorStore, GenerationId, IndexDocument, IndexDocumentId, VectorKey,
    EMBEDDING_JOB_SCHEMA_VERSION, INDEX_DOCUMENT_SCHEMA_VERSION,
};
use zlf_query::{
    BatchEmbeddingProvider, DurableEmbeddingWorker, EmbeddingJobStore, EmbeddingProviderFailure,
    IndexManifestStore,
};
use zlf_storage::Storage;

#[test]
#[allow(clippy::too_many_lines)]
fn worker_batches_transforms_normalizes_publishes_and_suppresses_stale_jobs() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage
        .create_node(Node::with_id(
            "doc".into(),
            vec!["document".into()],
            HashMap::from([("body".into(), Value::String("hello".into()))]),
        ))
        .unwrap();
    let state = storage
        .get_entity_state(&EntityRef::Node("doc".into()))
        .unwrap()
        .unwrap();
    let current_document = document("doc", state.source_version, "hello");
    IndexManifestStore::new(&storage, "vector:g1")
        .save(&manifest(current_document.clone()))
        .unwrap();
    let jobs = EmbeddingJobStore::new(&storage);
    jobs.enqueue(job(&current_document)).unwrap();
    let stale_document = document("stale", 1, "old");
    jobs.enqueue(job(&stale_document)).unwrap();

    let mut profile = bge_m3_dense_v1();
    profile.dimension = 2;
    profile.document_template = "doc: {text}".into();
    profile.query_template = "query: {text}".into();
    let exact = ExactVectorStore::open(temp.path().join("vectors")).unwrap();
    let provider = FakeProvider::default();
    let worker = DurableEmbeddingWorker::new(
        &storage,
        exact.clone(),
        &provider,
        profile.clone(),
        "vector:g1",
    )
    .unwrap();
    assert_eq!(worker.run_batch(Utc::now()).unwrap(), 1);
    assert_eq!(provider.documents.borrow().as_slice(), ["doc: hello"]);
    let record = exact.get(&vector_key("doc")).unwrap().unwrap();
    assert!((record.values[0] - 0.6).abs() < 1e-6);
    assert!((record.values[1] - 0.8).abs() < 1e-6);
    assert_eq!(
        jobs.get(&job(&current_document)).unwrap().unwrap().state,
        EmbeddingJobState::Completed
    );
    assert_eq!(
        jobs.get(&job(&stale_document)).unwrap().unwrap().state,
        EmbeddingJobState::Stale
    );
    assert_eq!(worker.embed_query("hello").unwrap(), vec![0.6, 0.8]);
    assert_eq!(provider.query.borrow().as_deref(), Some("query: hello"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn provider_failures_retry_then_dead_letter_without_storing_source_text() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage
        .create_node(Node::with_id("doc".into(), vec![], HashMap::new()))
        .unwrap();
    let version = storage
        .get_entity_state(&EntityRef::Node("doc".into()))
        .unwrap()
        .unwrap()
        .source_version;
    let document = document("doc", version, "secret source text");
    IndexManifestStore::new(&storage, "vector:g1")
        .save(&manifest(document.clone()))
        .unwrap();
    let jobs = EmbeddingJobStore::new(&storage);
    jobs.enqueue(job(&document)).unwrap();
    let mut profile = bge_m3_dense_v1();
    profile.dimension = 2;
    let exact = ExactVectorStore::open(temp.path().join("vectors")).unwrap();
    let provider = FailingProvider;
    let worker = DurableEmbeddingWorker::new(&storage, exact, &provider, profile, "vector:g1")
        .unwrap()
        .with_policy(Duration::seconds(1), 2);
    let now = Utc::now();
    assert_eq!(worker.run_batch(now).unwrap(), 0);
    assert_eq!(worker.run_batch(now + Duration::seconds(2)).unwrap(), 0);
    let stored = jobs.get(&job(&document)).unwrap().unwrap();
    assert_eq!(stored.state, EmbeddingJobState::Dead);
    assert_eq!(stored.last_error_class.as_deref(), Some("network_timeout"));
    let bytes = bincode::serialize(&stored).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("secret source text"));
}

#[derive(Default)]
struct FakeProvider {
    documents: RefCell<Vec<String>>,
    query: RefCell<Option<String>>,
}

impl BatchEmbeddingProvider for FakeProvider {
    fn embed_query(
        &self,
        _profile: &zlf_index::EmbeddingModelProfile,
        text: &str,
    ) -> Result<Vec<f32>, EmbeddingProviderFailure> {
        *self.query.borrow_mut() = Some(text.into());
        Ok(vec![3.0, 4.0])
    }

    fn embed_documents(
        &self,
        _profile: &zlf_index::EmbeddingModelProfile,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingProviderFailure> {
        self.documents.borrow_mut().extend_from_slice(texts);
        Ok(texts.iter().map(|_| vec![3.0, 4.0]).collect())
    }
}

struct FailingProvider;

impl BatchEmbeddingProvider for FailingProvider {
    fn embed_query(
        &self,
        _profile: &zlf_index::EmbeddingModelProfile,
        _text: &str,
    ) -> Result<Vec<f32>, EmbeddingProviderFailure> {
        unreachable!()
    }

    fn embed_documents(
        &self,
        _profile: &zlf_index::EmbeddingModelProfile,
        _texts: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingProviderFailure> {
        Err(EmbeddingProviderFailure {
            class: "network_timeout".into(),
            retryable: true,
        })
    }
}

fn document(id: &str, source_version: u64, content: &str) -> IndexDocument {
    IndexDocument {
        schema_version: INDEX_DOCUMENT_SCHEMA_VERSION,
        id: IndexDocumentId::new(EntityRef::Node(id.into()), "body", "0"),
        source_version,
        content_fingerprint: content_fingerprint(content),
        source_range: None,
        chunk_ordinal: 0,
        chunk_profile: "whole-v1".into(),
        language: Some("en".into()),
        content: content.into(),
    }
}

fn manifest(document: IndexDocument) -> DocumentManifest {
    DocumentManifest {
        entity: document.id.entity.clone(),
        profile_name: "knowledge".into(),
        profile_version: 1,
        source_version: document.source_version,
        documents: vec![document],
    }
}

fn job(document: &IndexDocument) -> EmbeddingJob {
    EmbeddingJob {
        schema_version: EMBEDDING_JOB_SCHEMA_VERSION,
        generation: GenerationId("g1".into()),
        document_id: document.id.clone(),
        source_version: document.source_version,
        content_fingerprint: document.content_fingerprint.clone(),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        expected_dimension: 2,
        attempts: 0,
        state: EmbeddingJobState::Pending,
        created_at: Utc::now(),
        lease_until: None,
        retry_at: None,
        completed_at: None,
        last_error_class: None,
    }
}

fn vector_key(id: &str) -> VectorKey {
    VectorKey {
        generation: GenerationId("g1".into()),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        document_id: IndexDocumentId::new(EntityRef::Node(id.into()), "body", "0"),
    }
}
