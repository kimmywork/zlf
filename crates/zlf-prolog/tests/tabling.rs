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
        PrologParser::parse_rule("reachable(X,Y) :- edge(X,Z), reachable(Z,Y).").unwrap(),
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
