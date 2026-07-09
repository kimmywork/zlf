use std::collections::HashMap;

use crate::PrologParser;
use zlf_core::{Edge, Node, Value};
use zlf_storage::Storage;

use super::super::{StaticFactProvider, StorageFactProvider, WamRuntime};

#[test]
fn runtime_queries_multi_fact_solutions() {
    let mut runtime = WamRuntime::new(8);
    runtime.add_fact(term("color(red)"));
    runtime.add_fact(term("color(green)"));
    runtime.add_fact(term("color(blue)"));

    let solutions = runtime.query_all(&term("color(X)")).unwrap();

    assert_eq!(solutions.len(), 3);
    assert_eq!(solutions[0].get("X"), Some(&atom("red")));
    assert_eq!(solutions[1].get("X"), Some(&atom("green")));
    assert_eq!(solutions[2].get("X"), Some(&atom("blue")));
}

#[test]
fn runtime_cut_commits_choices_created_after_predicate_call() {
    let mut runtime = WamRuntime::new(12);
    runtime.add_fact(term("color(red)"));
    runtime.add_fact(term("color(green)"));
    runtime.add_rule(rule("first_color(X) :- color(X), !."));

    let solutions = runtime.query_all(&term("first_color(X)")).unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("X"), Some(&atom("red")));
}

#[test]
fn runtime_queries_list_terms() {
    let mut runtime = WamRuntime::new(12);
    runtime.add_fact(term("tags(alice, [person, developer])"));
    runtime.add_rule(rule("has_tags(X, Tags) :- tags(X, Tags)."));

    let solutions = runtime.query_all(&term("has_tags(alice, Tags)")).unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(
        solutions[0].get("Tags"),
        Some(&crate::Term::List(vec![atom("person"), atom("developer")]))
    );
}

#[test]
fn runtime_queries_facts_from_provider() {
    let runtime = WamRuntime::new(8);
    let provider = StaticFactProvider::new(vec![
        term("color(red)"),
        term("color(green)"),
        term("color(blue)"),
    ]);

    let solutions = runtime
        .query_all_with_provider(&term("color(X)"), &provider)
        .unwrap();

    assert_eq!(solutions.len(), 3);
    assert_eq!(solutions[0].get("X"), Some(&atom("red")));
    assert_eq!(solutions[1].get("X"), Some(&atom("green")));
    assert_eq!(solutions[2].get("X"), Some(&atom("blue")));
}

#[test]
fn runtime_queries_rule_body_facts_from_provider() {
    let mut runtime = WamRuntime::new(12);
    runtime.add_rule(rule("grandparent(X, Z) :- parent(X, Y), parent(Y, Z)."));
    let provider =
        StaticFactProvider::new(vec![term("parent(alice, bob)"), term("parent(bob, carol)")]);

    let solutions = runtime
        .query_all_with_provider(&term("grandparent(alice, Who)"), &provider)
        .unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("Who"), Some(&atom("carol")));
}

#[test]
fn runtime_queries_storage_provider_properties_and_edges() {
    let storage = storage_fixture();
    let provider = StorageFactProvider::new(&storage);
    let runtime = WamRuntime::new(8);

    let names = runtime
        .query_all_with_provider(&term("property(alice, name, Name)"), &provider)
        .unwrap();
    let edges = runtime
        .query_all_with_provider(&term("edge(alice, knows, Target)"), &provider)
        .unwrap();

    assert_eq!(names[0].get("Name"), Some(&atom("Alice")));
    assert_eq!(edges[0].get("Target"), Some(&atom("bob")));
}

#[test]
fn runtime_queries_rule_body_from_storage_provider() {
    let storage = storage_fixture();
    let provider = StorageFactProvider::new(&storage);
    let mut runtime = WamRuntime::new(8);
    runtime.add_rule(rule("connected(X, Z) :- edge(X, knows, Z)."));

    let solutions = runtime
        .query_all_with_provider(&term("connected(alice, Who)"), &provider)
        .unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("Who"), Some(&atom("bob")));
}

#[test]
fn runtime_queries_rule_body_solution() {
    let mut runtime = WamRuntime::new(12);
    runtime.add_fact(term("parent(alice, bob)"));
    runtime.add_fact(term("parent(bob, carol)"));
    runtime.add_rule(rule("grandparent(X, Z) :- parent(X, Y), parent(Y, Z)."));

    let solutions = runtime.query_all(&term("grandparent(alice, Who)")).unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("Who"), Some(&atom("carol")));
}

#[test]
fn runtime_preserves_query_binding_across_higher_arity_rule_body_goal() {
    let mut runtime = WamRuntime::new(16);
    runtime.add_fact(term("knows(alice, bob)"));
    runtime.add_fact(term("property(bob, name, \"Bob\")"));
    runtime.add_rule(rule(
        "query_result(X) :- knows(alice, X), property(X, name, \"Bob\").",
    ));

    let solutions = runtime.query_all(&term("query_result(X)")).unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("X"), Some(&atom("bob")));
}

#[test]
fn runtime_preserves_bindings_after_nested_rule_call_in_rule_body() {
    let mut runtime = WamRuntime::new(24);
    runtime.add_fact(term("person(zlf)"));
    runtime.add_fact(term("person(tongtong)"));
    runtime.add_fact(term("knows(zlf, tongtong)"));
    runtime.add_fact(term("prop_name(zlf, \"峰哥亡命天涯\")"));
    runtime.add_fact(term("prop_name(tongtong, \"散仙彤彤子\")"));
    runtime.add_rule(rule("friend(X, Y) :- person(X), person(Y), knows(X, Y)."));
    runtime.add_rule(rule("q(X, Z) :- friend(zlf, X), prop_name(X, Z)."));

    let solutions = runtime.query_all(&term("q(X, Z)")).unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("X"), Some(&atom("tongtong")));
    assert_eq!(solutions[0].get("Z"), Some(&atom("散仙彤彤子")));
}

fn storage_fixture() -> Storage {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let storage = Storage::open(&path).unwrap();
    storage.create_node(node("alice", "Alice")).unwrap();
    storage.create_node(node("bob", "Bob")).unwrap();
    storage
        .create_edge(Edge::with_id(
            "edge-1".to_string(),
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        ))
        .unwrap();
    storage
}

fn node(id: &str, name: &str) -> Node {
    let mut props = HashMap::new();
    props.insert("name".to_string(), Value::String(name.to_string()));
    Node::with_id(id.to_string(), vec!["person".to_string()], props)
}

fn atom(value: &str) -> crate::Term {
    crate::Term::Atom(value.to_string())
}

fn term(source: &str) -> crate::Term {
    PrologParser::parse_term(source).unwrap()
}

fn rule(source: &str) -> crate::parser::PrologRule {
    PrologParser::parse_rule(source).unwrap()
}
