use zlf_prolog::wam::{PredicateKey, TableKey, TableState, WamRuntime};
use zlf_prolog::{PrologParser, Term};

#[test]
fn variant_calls_share_the_same_table_key() {
    let first = TableKey::from_call(&term("reachable(a, X)")).unwrap();
    let second = TableKey::from_call(&term("reachable(a, Y)")).unwrap();
    let different = TableKey::from_call(&term("reachable(b, X)")).unwrap();
    assert_eq!(first, second);
    assert_ne!(first, different);
}

#[test]
fn positive_recursive_tabling_terminates_on_cycles() {
    let runtime = cyclic_runtime();
    let query = term("reachable(a, X)");
    let key = TableKey::from_call(&query).unwrap();
    let rows = runtime.query_all(&query).unwrap();
    let mut values = rows.iter().map(|row| row["X"].clone()).collect::<Vec<_>>();
    values.sort_by_key(|value| format!("{value:?}"));
    assert_eq!(values, atoms(&["a", "b", "c", "d"]));
    assert_eq!(runtime.table_state(&key), Some(TableState::Complete));
    assert_eq!(runtime.query_all(&query).unwrap().len(), 4);
}

#[test]
fn table_answers_are_deduplicated_across_multiple_paths() {
    let mut runtime = WamRuntime::new(64);
    for edge in ["edge(a,b)", "edge(a,c)", "edge(b,d)", "edge(c,d)"] {
        runtime.add_fact(term(edge));
    }
    add_reachable_rules(&mut runtime);
    let rows = runtime.query_all(&term("reachable(a, X)")).unwrap();
    assert_eq!(rows.iter().filter(|row| row["X"] == atom("d")).count(), 1);
}

#[test]
fn right_recursive_transitive_rules_are_normalized_for_bound_calls() {
    let mut runtime = WamRuntime::new(64);
    for edge in ["edge(a,b)", "edge(b,c)", "edge(c,a)", "edge(c,d)"] {
        runtime.add_fact(term(edge));
    }
    runtime.add_rule(PrologParser::parse_rule("right_path(X,Y) :- edge(X,Y).").unwrap());
    runtime.add_rule(
        PrologParser::parse_rule("right_path(X,Y) :- edge(X,Z), right_path(Z,Y).").unwrap(),
    );
    runtime.declare_tabled(key("right_path", 2));
    let rows = runtime.query_all(&term("right_path(a,X)")).unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn nested_tabled_subgoals_join_complete_variant_answers() {
    let mut runtime = WamRuntime::new(64);
    for edge in ["edge(a,c)", "edge(b,c)", "edge(c,d)"] {
        runtime.add_fact(term(edge));
    }
    add_reachable_rules(&mut runtime);
    runtime.add_rule(
        PrologParser::parse_rule("common_reachable(A,B,X) :- reachable(A,X), reachable(B,X).")
            .unwrap(),
    );
    runtime.declare_tabled(key("common_reachable", 3));
    let rows = runtime.query_all(&term("common_reachable(a,b,X)")).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row["X"] == atom("c")));
    assert!(rows.iter().any(|row| row["X"] == atom("d")));
}

#[test]
fn completed_nested_tables_support_deterministic_cut_consumers() {
    let mut runtime = WamRuntime::new(64);
    for edge in ["parent(a,p)", "parent(b,p)", "parent(p,r)"] {
        runtime.add_fact(term(edge));
    }
    runtime.add_fact(term("distance_up(a,a,0)"));
    runtime.add_fact(term("distance_up(b,b,0)"));
    runtime.add_rule(
        PrologParser::parse_rule(
            "distance_up(S,Y,D) :- distance_up(S,X,D0), parent(X,Y), is(D,'+'(D0,1)).",
        )
        .unwrap(),
    );
    runtime.add_rule(
        PrologParser::parse_rule("tree_lca(A,B,L) :- distance_up(A,L,DA), distance_up(B,L,DB), !.")
            .unwrap(),
    );
    runtime.declare_tabled(key("distance_up", 3));
    runtime.declare_tabled(key("tree_lca", 3));
    let rows = runtime.query_all(&term("tree_lca(a,b,L)")).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["L"], atom("p"));
}

#[test]
fn mutually_recursive_tabled_predicates_complete_as_one_component() {
    let mut runtime = WamRuntime::new(32);
    runtime.add_fact(term("even(a)"));
    runtime.add_rule(PrologParser::parse_rule("odd(X) :- even(X).").unwrap());
    runtime.add_rule(PrologParser::parse_rule("even(X) :- odd(X).").unwrap());
    runtime.declare_tabled(key("even", 1));
    runtime.declare_tabled(key("odd", 1));
    let rows = runtime.query_all(&term("odd(X)")).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["X"], atom("a"));
}

#[test]
fn unsupported_negative_tabled_rules_are_rejected() {
    let mut runtime = WamRuntime::new(32);
    runtime.add_rule(zlf_prolog::PrologRule {
        head: compound("unsafe", vec![var("X")]),
        body: vec![compound("\\+", vec![compound("blocked", vec![var("X")])])],
    });
    runtime.declare_tabled(key("unsafe", 1));
    assert!(runtime.query_all(&term("unsafe(X)")).is_err());
}

fn cyclic_runtime() -> WamRuntime {
    let mut runtime = WamRuntime::new(64);
    for edge in ["edge(a,b)", "edge(b,c)", "edge(c,a)", "edge(c,d)"] {
        runtime.add_fact(term(edge));
    }
    add_reachable_rules(&mut runtime);
    runtime
}

fn add_reachable_rules(runtime: &mut WamRuntime) {
    runtime.add_rule(PrologParser::parse_rule("reachable(X,Y) :- edge(X,Y).").unwrap());
    runtime.add_rule(
        PrologParser::parse_rule("reachable(X,Y) :- reachable(X,Z), edge(Z,Y).").unwrap(),
    );
    runtime.declare_tabled(key("reachable", 2));
}

fn key(name: &str, arity: usize) -> PredicateKey {
    PredicateKey {
        name: name.to_string(),
        arity,
    }
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}

fn var(value: &str) -> Term {
    Term::Variable(value.to_string())
}

fn compound(name: &str, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.to_string(),
        args,
    }
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}

fn atoms(values: &[&str]) -> Vec<Term> {
    values.iter().map(|value| atom(value)).collect()
}
