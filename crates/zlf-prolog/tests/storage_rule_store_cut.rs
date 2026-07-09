use std::collections::HashMap;

use zlf_core::{Edge, Node, Value};
use zlf_prolog::wam::{StorageFactProvider, StorageRuleStore, WamRuntime};
use zlf_prolog::{PrologParser, Term};
use zlf_storage::Storage;

#[test]
fn storage_rule_store_persists_cut_rules_for_runtime() {
    let storage = storage_fixture();
    let store = StorageRuleStore::new(&storage);
    let rule = rule("first_person(X) :- person(X), !.");
    let key = zlf_prolog::wam::predicate_key(&rule.head).unwrap();

    store.add_rule(&rule).unwrap();
    let loaded = store.rules_for(&key).unwrap();
    let provider = StorageFactProvider::new(&storage);
    let mut runtime = WamRuntime::new(12);
    for artifact in loaded {
        runtime.add_compiled_rule(artifact);
    }

    let solutions = runtime
        .query_all_with_provider(&term("first_person(X)"), &provider)
        .unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("X"), Some(&atom("alice")));
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

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}

fn rule(source: &str) -> zlf_prolog::parser::PrologRule {
    PrologParser::parse_rule(source).unwrap()
}
