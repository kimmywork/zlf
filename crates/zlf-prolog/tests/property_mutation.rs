use std::collections::HashMap;

use zlf_core::{Edge, Node, Value};
use zlf_prolog::wam::{CompositeFactProvider, StorageFactProvider, WamRuntime};
use zlf_prolog::Term;
use zlf_storage::Storage;

#[test]
#[allow(clippy::too_many_lines)]
fn explicit_property_builtins_mutate_nodes_and_edges() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("storage")).unwrap();
    storage
        .create_node(Node::with_id("alice".into(), vec![], HashMap::new()))
        .unwrap();
    storage
        .create_node(Node::with_id("bob".into(), vec![], HashMap::new()))
        .unwrap();
    storage
        .create_edge(Edge::with_id(
            "e1".into(),
            "knows".into(),
            "alice".into(),
            "bob".into(),
            HashMap::new(),
        ))
        .unwrap();
    let provider_storage = StorageFactProvider::new(&storage);
    let provider = CompositeFactProvider::new().with(&provider_storage);
    let runtime = WamRuntime::new(32);

    assert_eq!(
        run(
            &runtime,
            &provider,
            &storage,
            compound(
                "set_node_property",
                vec![atom("alice"), atom("name"), string("Alice")],
            ),
        )
        .len(),
        1
    );
    assert_eq!(
        run(
            &runtime,
            &provider,
            &storage,
            compound(
                "set_edge_property",
                vec![atom("e1"), atom("confidence"), Term::Float(0.9)],
            ),
        )
        .len(),
        1
    );
    assert_eq!(
        storage.get_node("alice").unwrap().unwrap().properties["name"],
        Value::String("Alice".into())
    );
    assert_eq!(
        storage.get_edge("e1").unwrap().unwrap().properties["confidence"],
        Value::Number(0.9)
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn generic_property_assert_and_retract_resolve_edges() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("storage")).unwrap();
    storage
        .create_node(Node::with_id("alice".into(), vec![], HashMap::new()))
        .unwrap();
    storage
        .create_node(Node::with_id("bob".into(), vec![], HashMap::new()))
        .unwrap();
    storage
        .create_edge(Edge::with_id(
            "e1".into(),
            "knows".into(),
            "alice".into(),
            "bob".into(),
            HashMap::new(),
        ))
        .unwrap();
    let provider_storage = StorageFactProvider::new(&storage);
    let provider = CompositeFactProvider::new().with(&provider_storage);
    let runtime = WamRuntime::new(32);
    let property = compound(
        "property",
        vec![atom("e1"), atom("confidence"), Term::Float(0.8)],
    );
    assert_eq!(
        run(
            &runtime,
            &provider,
            &storage,
            compound("assertz", vec![property.clone()]),
        )
        .len(),
        1
    );
    assert_eq!(
        storage.get_edge("e1").unwrap().unwrap().properties["confidence"],
        Value::Number(0.8)
    );
    assert_eq!(
        run(
            &runtime,
            &provider,
            &storage,
            compound("retract", vec![property]),
        )
        .len(),
        1
    );
    assert!(!storage
        .get_edge("e1")
        .unwrap()
        .unwrap()
        .properties
        .contains_key("confidence"));
}

#[test]
fn edge_id_relation_returns_stable_id() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("storage")).unwrap();
    storage
        .create_node(Node::with_id("alice".into(), vec![], HashMap::new()))
        .unwrap();
    storage
        .create_node(Node::with_id("bob".into(), vec![], HashMap::new()))
        .unwrap();
    storage
        .create_edge(Edge::with_id(
            "edge-1".into(),
            "knows".into(),
            "alice".into(),
            "bob".into(),
            HashMap::new(),
        ))
        .unwrap();
    let provider_storage = StorageFactProvider::new(&storage);
    let provider = CompositeFactProvider::new().with(&provider_storage);
    let runtime = WamRuntime::new(32);
    let rows = run(
        &runtime,
        &provider,
        &storage,
        compound(
            "edge_id",
            vec![atom("alice"), atom("knows"), atom("bob"), var("Id")],
        ),
    );
    assert_eq!(rows[0]["Id"], atom("edge-1"));
}

fn run(
    runtime: &WamRuntime,
    provider: &CompositeFactProvider<'_>,
    storage: &Storage,
    term: Term,
) -> Vec<HashMap<String, Term>> {
    runtime
        .query_all_with_provider_and_storage(&term, provider, storage)
        .unwrap()
}

fn atom(value: &str) -> Term {
    Term::Atom(value.into())
}

fn string(value: &str) -> Term {
    Term::String(value.into())
}

fn var(value: &str) -> Term {
    Term::Variable(value.into())
}

fn compound(name: &str, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}
