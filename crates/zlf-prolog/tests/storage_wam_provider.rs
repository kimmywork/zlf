use std::collections::HashMap;

use zlf_core::{Edge, Node, Value};
use zlf_index::{BM25Index, VectorIndex};
use zlf_prolog::wam::{
    CompositeFactProvider, Embedder, EmbeddingQueue, EmbeddingWorker, IndexFactProvider,
    IndexedStorageFactWriter, PersistentEmbeddingQueue, StorageFactProvider, StorageFactWriter,
    StorageRuleStore, WamRuntime,
};
use zlf_prolog::{PrologParser, Term};
use zlf_storage::Storage;

#[test]
fn storage_writer_persists_node_object_literal_to_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep().join("db");
    let storage = Storage::open(&path).unwrap();
    let writer = StorageFactWriter::new(&storage);

    writer
        .apply_fact(&term("node(alice, [person], { name: \"Alice\", age: 17 })"))
        .unwrap();
    writer
        .apply_fact(&term("knows(alice, bob, { since: 2020 })"))
        .unwrap();

    let alice = storage.get_node("alice").unwrap().unwrap();
    let edge = storage.get_edge("alice:knows:bob").unwrap().unwrap();

    assert!(alice.has_label("person"));
    assert_eq!(
        alice.get_property("name"),
        Some(&Value::String("Alice".to_string()))
    );
    assert_eq!(alice.get_property("age"), Some(&Value::Number(17.0)));
    assert_eq!(edge.get_property("since"), Some(&Value::Number(2020.0)));
}

#[test]
fn bm25_provider_reads_backend_documents() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let bm25_path = dir.path().join("bm25");
    let storage = Storage::open(&path).unwrap();
    let bm25 = BM25Index::open(&bm25_path).unwrap();
    let writer = IndexedStorageFactWriter::new(&storage);
    writer
        .apply_fact(&term("node(doc1, [document], { title: \"软件工程师\" })"))
        .unwrap();
    bm25.index_text("doc1", "软件工程师").unwrap();

    let provider = IndexFactProvider::new().with_bm25(&bm25);
    let runtime = WamRuntime::new(12);
    let solutions = runtime
        .query_all_with_provider(&term("bm25(\"软件\", Node, Score)"), &provider)
        .unwrap();

    assert_eq!(solutions[0].get("Node"), Some(&atom("doc1")));
    assert!(solutions[0].contains_key("Score"));
}

#[test]
fn composite_provider_combines_storage_and_index_queries() {
    let dir = tempfile::tempdir().unwrap();
    let storage_path = dir.path().join("db");
    let bm25_path = dir.path().join("bm25");
    let storage = Storage::open(&storage_path).unwrap();
    let bm25 = BM25Index::open(&bm25_path).unwrap();
    let writer = IndexedStorageFactWriter::new(&storage);
    writer
        .apply_fact(&term("node(doc1, [document], { title: \"软件工程师\" })"))
        .unwrap();
    bm25.index_text("doc1", "软件工程师").unwrap();
    let storage_provider = StorageFactProvider::new(&storage);
    let index_provider = IndexFactProvider::new().with_bm25(&bm25);
    let provider = CompositeFactProvider::new()
        .with(&storage_provider)
        .with(&index_provider);
    let mut runtime = WamRuntime::new(16);
    runtime.add_rule(rule(
        "search_doc(X, Title, Score) :- document(X), prop_title(X, Title), bm25(\"软件\", X, Score).",
    ));

    let solutions = runtime
        .query_all_with_provider(&term("search_doc(doc1, Title, Score)"), &provider)
        .unwrap();

    assert_eq!(solutions[0].get("Title"), Some(&string("软件工程师")));
    assert!(solutions[0].contains_key("Score"));
}

#[test]
fn embedding_worker_loop_processes_persistent_queue_until_idle() {
    let dir = tempfile::tempdir().unwrap();
    let storage_path = dir.path().join("db");
    let vector_path = dir.path().join("vector");
    let storage = Storage::open(&storage_path).unwrap();
    let vector = VectorIndex::open(&vector_path).unwrap();
    let embedder = FakeEmbedder;
    let queue = PersistentEmbeddingQueue::new(&storage);
    queue.enqueue("doc1", "软件工程师").unwrap();
    let worker = EmbeddingWorker::new(&queue, &embedder, &vector)
        .with_poll_interval(std::time::Duration::from_millis(1));

    assert_eq!(worker.run_until_idle(1).unwrap(), 1);
    assert!(queue.pending().unwrap().is_empty());
}

#[test]
fn persistent_embedding_queue_survives_storage_and_processes_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let storage_path = dir.path().join("db");
    let vector_path = dir.path().join("vector");
    let storage = Storage::open(&storage_path).unwrap();
    let vector = VectorIndex::open(&vector_path).unwrap();
    let embedder = FakeEmbedder;
    let queue = PersistentEmbeddingQueue::new(&storage);

    queue.enqueue("doc1", "软件工程师").unwrap();
    assert_eq!(queue.pending().unwrap().len(), 1);
    assert_eq!(queue.process_all(&embedder, &vector).unwrap(), 1);
    assert!(queue.pending().unwrap().is_empty());

    let provider = IndexFactProvider::new().with_vector(&vector);
    let runtime = WamRuntime::new(12);
    let solutions = runtime
        .query_all_with_provider(&term("vector_similar(doc1, Node, Score)"), &provider)
        .unwrap();
    assert_eq!(solutions[0].get("Node"), Some(&atom("doc1")));
}

#[test]
fn embedding_queue_processes_text_jobs_into_vector_index() {
    let dir = tempfile::tempdir().unwrap();
    let vector_path = dir.path().join("vector");
    let vector = VectorIndex::open(&vector_path).unwrap();
    let embedder = FakeEmbedder;
    let mut queue = EmbeddingQueue::new();

    queue
        .enqueue_fact(&term("node(doc1, [document], { title: \"软件工程师\" })"))
        .unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.process_all(&embedder, &vector).unwrap(), 1);

    let provider = IndexFactProvider::new().with_vector(&vector);
    let runtime = WamRuntime::new(12);
    let solutions = runtime
        .query_all_with_provider(&term("vector_similar(doc1, Node, Score)"), &provider)
        .unwrap();

    assert_eq!(solutions[0].get("Node"), Some(&atom("doc1")));
}

#[test]
fn indexed_writer_updates_embedding_vector_for_text_properties() {
    let dir = tempfile::tempdir().unwrap();
    let storage_path = dir.path().join("db");
    let vector_path = dir.path().join("vector");
    let storage = Storage::open(&storage_path).unwrap();
    let vector = VectorIndex::open(&vector_path).unwrap();
    let embedder = FakeEmbedder;
    let writer = IndexedStorageFactWriter::new(&storage).with_embedding(&vector, &embedder);
    writer
        .apply_fact(&term("node(doc1, [document], { title: \"软件工程师\" })"))
        .unwrap();
    let provider = IndexFactProvider::new().with_vector(&vector);
    let runtime = WamRuntime::new(12);

    let solutions = runtime
        .query_all_with_provider(&term("vector_similar(doc1, Node, Score)"), &provider)
        .unwrap();

    assert_eq!(solutions[0].get("Node"), Some(&atom("doc1")));
    assert!(solutions[0].contains_key("Score"));
}

#[allow(clippy::too_many_lines)]
#[test]
fn storage_writer_persists_prolog_facts_to_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep().join("db");
    let storage = Storage::open(&path).unwrap();
    let writer = StorageFactWriter::new(&storage);

    writer.apply_fact(&term("node(alice)")).unwrap();
    writer.apply_fact(&term("node(bob)")).unwrap();
    writer.apply_fact(&term("knows(alice, bob)")).unwrap();
    writer
        .apply_fact(&term("prop_name(alice, \"Alice\")"))
        .unwrap();

    let alice = storage.get_node("alice").unwrap().unwrap();
    let bob = storage.get_node("bob").unwrap().unwrap();
    let edge = storage.get_edge("alice:knows:bob").unwrap().unwrap();
    assert_eq!(
        alice.get_property("name"),
        Some(&Value::String("Alice".to_string()))
    );
    assert_eq!(bob.id, "bob");
    assert_eq!(edge.source, "alice");
    assert_eq!(edge.edge_type, "knows");
    assert_eq!(edge.target, "bob");

    let provider = StorageFactProvider::new(&storage);
    let runtime = WamRuntime::new(12);
    let edges = runtime
        .query_all_with_provider(&term("knows(alice, Target)"), &provider)
        .unwrap();
    let names = runtime
        .query_all_with_provider(&term("prop_name(alice, Name)"), &provider)
        .unwrap();

    assert_eq!(edges[0].get("Target"), Some(&atom("bob")));
    assert_eq!(names[0].get("Name"), Some(&string("Alice")));
}

#[test]
fn storage_writer_persists_edge_triples_to_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep().join("db");
    let storage = Storage::open(&path).unwrap();
    let writer = StorageFactWriter::new(&storage);

    writer.apply_fact(&term("edge(alice, knows, bob)")).unwrap();

    assert!(storage.get_node("alice").unwrap().is_some());
    assert!(storage.get_node("bob").unwrap().is_some());
    assert!(storage.get_edge("alice:knows:bob").unwrap().is_some());
    assert_eq!(storage.get_edges_by_type("knows").unwrap().len(), 1);

    let provider = StorageFactProvider::new(&storage);
    let runtime = WamRuntime::new(12);
    let edges = runtime
        .query_all_with_provider(&term("edge(alice, knows, Target)"), &provider)
        .unwrap();

    assert_eq!(edges[0].get("Target"), Some(&atom("bob")));
}

#[test]
fn storage_rule_store_persists_and_loads_rules_for_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep().join("db");
    let storage = Storage::open(&path).unwrap();
    let store = StorageRuleStore::new(&storage);
    let rule = rule("friend(X, Y) :- knows(X, Y).");
    let key = zlf_prolog::wam::predicate_key(&rule.head).unwrap();

    store.add_rule(&rule).unwrap();
    let loaded = store.rules_for(&key).unwrap();
    assert_eq!(loaded.len(), 1);

    let writer = StorageFactWriter::new(&storage);
    writer.apply_fact(&term("knows(alice, bob)")).unwrap();
    let provider = StorageFactProvider::new(&storage);
    let mut runtime = WamRuntime::new(12);
    assert!(!loaded[0].program.instructions().is_empty());
    for artifact in loaded {
        runtime.add_compiled_rule(artifact);
    }
    let solutions = runtime
        .query_all_with_provider(&term("friend(alice, Who)"), &provider)
        .unwrap();

    assert_eq!(solutions[0].get("Who"), Some(&atom("bob")));
}

#[test]
fn storage_provider_reads_properties_edges_and_edge_type_shortcuts() {
    let storage = storage_fixture();
    let provider = StorageFactProvider::new(&storage);
    let runtime = WamRuntime::new(12);

    let names = runtime
        .query_all_with_provider(&term("property(alice, name, Name)"), &provider)
        .unwrap();
    let edges = runtime
        .query_all_with_provider(&term("edge(alice, knows, Target)"), &provider)
        .unwrap();
    let shortcuts = runtime
        .query_all_with_provider(&term("knows(alice, Target)"), &provider)
        .unwrap();

    assert_eq!(names[0].get("Name"), Some(&string("Alice")));
    assert_eq!(edges[0].get("Target"), Some(&atom("bob")));
    assert_eq!(shortcuts[0].get("Target"), Some(&atom("bob")));
}

#[test]
fn storage_provider_supports_label_and_property_shortcuts() {
    let storage = storage_fixture();
    let provider = StorageFactProvider::new(&storage);
    let runtime = WamRuntime::new(12);

    let labels = runtime
        .query_all_with_provider(&term("person(alice)"), &provider)
        .unwrap();
    let names = runtime
        .query_all_with_provider(&term("prop_name(alice, Name)"), &provider)
        .unwrap();

    assert_eq!(labels.len(), 1);
    assert_eq!(names[0].get("Name"), Some(&string("Alice")));
}

#[test]
fn storage_provider_shortcuts_work_inside_rule_body() {
    let storage = storage_fixture();
    let provider = StorageFactProvider::new(&storage);
    let mut runtime = WamRuntime::new(12);
    runtime.add_rule(rule(
        "named_person(X, Name) :- person(X), prop_name(X, Name).",
    ));

    let solutions = runtime
        .query_all_with_provider(&term("named_person(alice, Name)"), &provider)
        .unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("Name"), Some(&string("Alice")));
}

#[test]
fn storage_provider_feeds_rule_body_predicates_from_database() {
    let storage = storage_fixture();
    let provider = StorageFactProvider::new(&storage);
    let mut runtime = WamRuntime::new(12);
    runtime.add_rule(rule("friend(X, Y) :- knows(X, Y)."));

    let solutions = runtime
        .query_all_with_provider(&term("friend(alice, Who)"), &provider)
        .unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("Who"), Some(&atom("bob")));
}

fn storage_fixture() -> Storage {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep().join("db");
    let storage = Storage::open(&path).unwrap();
    storage.create_node(node("alice", "Alice")).unwrap();
    storage.create_node(node("bob", "Bob")).unwrap();
    storage.create_edge(edge("knows", "alice", "bob")).unwrap();
    storage
}

fn node(id: &str, name: &str) -> Node {
    let mut props = HashMap::new();
    props.insert("name".to_string(), Value::String(name.to_string()));
    Node::with_id(id.to_string(), vec!["person".to_string()], props)
}

fn edge(edge_type: &str, source: &str, target: &str) -> Edge {
    Edge::with_id(
        format!("{source}-{edge_type}-{target}"),
        edge_type.to_string(),
        source.to_string(),
        target.to_string(),
        HashMap::new(),
    )
}

fn string(value: &str) -> Term {
    Term::String(value.to_string())
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}

fn rule(source: &str) -> zlf_prolog::PrologRule {
    PrologParser::parse_rule(source).unwrap()
}

struct FakeEmbedder;

impl Embedder for FakeEmbedder {
    fn model(&self) -> &str {
        "fake"
    }

    fn embed(&self, text: &str) -> zlf_prolog::wam::WamResult<Vec<f32>> {
        if text.contains("软件") {
            Ok(vec![1.0, 0.0])
        } else {
            Ok(vec![0.0, 1.0])
        }
    }
}
