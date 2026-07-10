use std::collections::HashMap;

use zlf_core::{Edge, Node, Value};
use zlf_prolog::PrologParser;
use zlf_query::ZlfDatabase;

fn db() -> ZlfDatabase {
    let path = tempfile::tempdir().unwrap().keep();
    ZlfDatabase::open(&path).unwrap()
}

#[allow(clippy::too_many_lines)]
fn seed_graph(db: &ZlfDatabase) {
    db.add_node(Node::with_id(
        "alice".to_string(),
        vec!["person".to_string()],
        [("name".to_string(), Value::String("Alice".to_string()))].into(),
    ))
    .unwrap();
    db.add_node(Node::with_id(
        "bob".to_string(),
        vec!["person".to_string()],
        HashMap::new(),
    ))
    .unwrap();
    db.add_node(Node::with_id(
        "carol".to_string(),
        vec!["person".to_string()],
        HashMap::new(),
    ))
    .unwrap();
    db.add_edge(Edge::new(
        "knows".to_string(),
        "alice".to_string(),
        "bob".to_string(),
        HashMap::new(),
    ))
    .unwrap();
    db.add_edge(Edge::new(
        "knows".to_string(),
        "bob".to_string(),
        "carol".to_string(),
        HashMap::new(),
    ))
    .unwrap();
}

#[test]
#[allow(clippy::too_many_lines)]
fn stage0_retract_removes_label_property_and_edge_visibility() {
    let db = db();
    db.apply_fact(
        &PrologParser::parse_fact("node(alice, [person], { name: \"Alice\" }).")
            .unwrap()
            .head,
    )
    .unwrap();
    db.apply_fact(&PrologParser::parse_fact("knows(alice, bob).").unwrap().head)
        .unwrap();

    assert_eq!(db.query_prolog("? person(X).").unwrap()[0]["X"], "alice");
    assert_eq!(
        db.query_prolog("? prop_name(alice, Name).").unwrap()[0]["Name"],
        "Alice"
    );
    assert_eq!(
        db.query_prolog("? knows(alice, X).").unwrap()[0]["X"],
        "bob"
    );

    assert_eq!(
        db.query_prolog("? retract(person(alice)).").unwrap().len(),
        1
    );
    assert!(db.query_prolog("? person(X).").unwrap().is_empty());

    assert_eq!(
        db.query_prolog("? retract(prop_name(alice, _)).")
            .unwrap()
            .len(),
        1
    );
    assert!(db
        .query_prolog("? prop_name(alice, Name).")
        .unwrap()
        .is_empty());

    assert_eq!(
        db.query_prolog("? retract(edge(alice, knows, bob)).")
            .unwrap()
            .len(),
        1
    );
    assert!(db.query_prolog("? knows(alice, X).").unwrap().is_empty());
}

#[test]
fn stage1_introspection_returns_rule_source_and_dependencies() {
    let db = db();
    db.query_prolog("friend(X, Y) :- knows(X, Y).").unwrap();

    let rules = db.query_prolog("? rule(friend, 2, Source).").unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["Source"], "friend(X, Y) :- knows(X, Y).");

    let deps = db
        .query_prolog("? rule_depends_on(\"friend/2\", Dep).")
        .unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0]["Dep"], "knows/2");
}

#[test]
fn stage2_graph_view_predicates_return_typed_edges_and_node_view_objects() {
    let db = db();
    seed_graph(&db);

    let labels = db.query_prolog("? labels(alice, Labels).").unwrap();
    assert_eq!(labels[0]["Labels"], serde_json::json!(["person"]));

    let props = db.query_prolog("? properties(alice, Props).").unwrap();
    assert_eq!(props[0]["Props"]["name"], "Alice");

    let out = db
        .query_prolog("? out_edges(alice, knows, Edges).")
        .unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["Edges"][0]["source"], "alice");
    assert_eq!(out[0]["Edges"][0]["type"], "knows");
    assert_eq!(out[0]["Edges"][0]["target"], "bob");

    let neighbors = db.query_prolog("? neighbors(alice, knows, N).").unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0]["N"], "bob");

    let view = db.query_prolog("? node_view(alice, View).").unwrap();
    assert_eq!(view[0]["View"]["id"], "alice");
    assert_eq!(view[0]["View"]["out_edges"][0]["target"], "bob");
}

#[test]
fn stage3_graph_algorithms_work_with_uuid_edge_ids() {
    let db = db();
    seed_graph(&db);

    let reachable = db.query_prolog("? reachable(alice, X).").unwrap();
    let targets: Vec<_> = reachable.iter().map(|row| row["X"].clone()).collect();
    assert_eq!(
        targets,
        vec![serde_json::json!("bob"), serde_json::json!("carol")]
    );

    let bounded = db.query_prolog("? reachable(alice, X, 1).").unwrap();
    assert_eq!(bounded.len(), 1);
    assert_eq!(bounded[0]["X"], "bob");

    let path = db
        .query_prolog("? shortest_path(alice, carol, Path).")
        .unwrap();
    assert_eq!(
        path[0]["Path"],
        serde_json::json!(["alice", "bob", "carol"])
    );

    let degree = db.query_prolog("? out_degree(alice, D).").unwrap();
    assert_eq!(degree[0]["D"], serde_json::json!(1));
}

#[test]
fn stage3_shortest_path_enumerates_variable_targets_for_joins() {
    let db = db();
    for id in ["a", "b", "c"] {
        db.apply_fact(
            &PrologParser::parse_fact(&format!("node({id})."))
                .unwrap()
                .head,
        )
        .unwrap();
    }
    db.apply_fact(&PrologParser::parse_fact("follows(b, a).").unwrap().head)
        .unwrap();
    db.apply_fact(&PrologParser::parse_fact("follows(c, b).").unwrap().head)
        .unwrap();
    db.query_prolog("after(X, Y) :- follows(X, Y).").unwrap();
    db.query_prolog("after(X, Y) :- follows(X, C), after(C, Y).")
        .unwrap();

    let paths = db.query_prolog("? shortest_path(c, X, P).").unwrap();
    let targets: Vec<_> = paths.iter().map(|row| row["X"].clone()).collect();
    assert_eq!(
        targets,
        vec![serde_json::json!("b"), serde_json::json!("a")]
    );

    let joined = db
        .query_prolog("? after(c, X), shortest_path(c, X, P).")
        .unwrap();
    assert_eq!(joined.len(), 2);
    assert_eq!(joined[0]["P"], serde_json::json!(["c", "b"]));
    assert_eq!(joined[1]["P"], serde_json::json!(["c", "b", "a"]));
}

#[test]
fn stage4_iso_core_arithmetic_univ_and_list_tail_queries() {
    let db = db();

    let arithmetic = db.query_prolog("? X is 1 + 2 * 3.").unwrap();
    assert_eq!(arithmetic.len(), 1);
    assert_eq!(arithmetic[0]["X"], serde_json::json!(7));

    let comparison = db.query_prolog("? 7 =:= 1 + 2 * 3.").unwrap();
    assert_eq!(comparison.len(), 1);

    let functor = db
        .query_prolog("? functor(parent(alice,bob), Name, Arity).")
        .unwrap();
    assert_eq!(functor[0]["Name"], "parent");
    assert_eq!(functor[0]["Arity"], serde_json::json!(2));

    let univ = db.query_prolog("? parent(alice,bob) =.. L.").unwrap();
    assert_eq!(univ[0]["L"], serde_json::json!(["parent", "alice", "bob"]));

    let list_tail = db.query_prolog("? [H|T] = [a,b,c].").unwrap();
    assert_eq!(list_tail[0]["H"], "a");
    assert_eq!(list_tail[0]["T"], serde_json::json!(["b", "c"]));
}

#[test]
fn stage4_list_library_and_conversion_subset() {
    let db = db();

    let members = db.query_prolog("? member(X, [a,b,c]).").unwrap();
    let values: Vec<_> = members.iter().map(|row| row["X"].clone()).collect();
    assert_eq!(
        values,
        vec![
            serde_json::json!("a"),
            serde_json::json!("b"),
            serde_json::json!("c")
        ]
    );

    let appended = db.query_prolog("? append([a,b], [c], X).").unwrap();
    assert_eq!(appended[0]["X"], serde_json::json!(["a", "b", "c"]));

    let length = db.query_prolog("? length([a,b,c], N).").unwrap();
    assert_eq!(length[0]["N"], serde_json::json!(3));

    let reversed = db.query_prolog("? reverse([a,b,c], R).").unwrap();
    assert_eq!(reversed[0]["R"], serde_json::json!(["c", "b", "a"]));

    let selected = db.query_prolog("? select(b, [a,b,c], R).").unwrap();
    assert_eq!(selected[0]["R"], serde_json::json!(["a", "c"]));

    let nth0 = db.query_prolog("? nth0(1, [a,b,c], X).").unwrap();
    assert_eq!(nth0[0]["X"], "b");

    let chars = db.query_prolog("? string_chars(\"ab\", Cs).").unwrap();
    assert_eq!(chars[0]["Cs"], serde_json::json!(["a", "b"]));

    let string = db.query_prolog("? atom_string(alice, S).").unwrap();
    assert_eq!(string[0]["S"], "alice");
}

#[test]
fn stage4_control_and_meta_call_subset() {
    let db = db();
    db.apply_fact(&PrologParser::parse_fact("knows(alice, bob).").unwrap().head)
        .unwrap();

    let call_goal = db.query_prolog("? call(knows(alice, bob)).").unwrap();
    assert_eq!(call_goal.len(), 1);

    let call_n = db.query_prolog("? call(knows, alice, X).").unwrap();
    assert_eq!(call_n.len(), 1);
    assert_eq!(call_n[0]["X"], "bob");

    let once = db.query_prolog("? once(member(X, [a,b,c])).").unwrap();
    assert_eq!(once.len(), 1);
    assert_eq!(once[0]["X"], "a");

    assert_eq!(
        db.query_prolog("? \\+ knows(alice, carol).").unwrap().len(),
        1
    );
    assert!(db
        .query_prolog("? \\+ knows(alice, bob).")
        .unwrap()
        .is_empty());

    let either = db
        .query_prolog("? member(X, [a]); member(X, [b]).")
        .unwrap();
    let values: Vec<_> = either.iter().map(|row| row["X"].clone()).collect();
    assert_eq!(values, vec![serde_json::json!("a"), serde_json::json!("b")]);

    let if_then = db.query_prolog("? knows(alice, bob) -> true.").unwrap();
    assert_eq!(if_then.len(), 1);
}

#[test]
fn stage4_term_identity_order_and_dynamic_db_subset() {
    let db = db();

    assert_eq!(db.query_prolog("? alice == alice.").unwrap().len(), 1);
    assert!(db.query_prolog("? alice == bob.").unwrap().is_empty());
    assert_eq!(db.query_prolog("? alice \\== bob.").unwrap().len(), 1);
    assert_eq!(db.query_prolog("? alice @< bob.").unwrap().len(), 1);

    assert_eq!(
        db.query_prolog("? assertz(likes(alice, tea)).")
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        db.query_prolog("? likes(alice, X).").unwrap()[0]["X"],
        "tea"
    );
    assert_eq!(
        db.query_prolog("? current_predicate(\"likes/2\").")
            .unwrap()
            .len(),
        1
    );

    assert_eq!(
        db.query_prolog("? retractall(likes(alice, tea)).")
            .unwrap()
            .len(),
        1
    );
    assert!(db.query_prolog("? likes(alice, X).").unwrap().is_empty());
}

#[test]
fn stage4_dynamic_db_supports_structured_indicators_and_retractall_patterns() {
    let db = db();
    db.query_prolog("? assertz(likes(alice, tea)).").unwrap();
    db.query_prolog("? assertz(likes(alice, coffee)).").unwrap();

    assert_eq!(
        db.query_prolog("? current_predicate(likes/2).")
            .unwrap()
            .len(),
        1
    );
    assert_eq!(db.query_prolog("? likes(alice, X).").unwrap().len(), 2);

    assert_eq!(
        db.query_prolog("? retractall(likes(alice, _)).")
            .unwrap()
            .len(),
        1
    );
    assert!(db.query_prolog("? likes(alice, X).").unwrap().is_empty());
}
