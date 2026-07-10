use std::collections::HashMap;

use zlf_prolog::wam::WamRuntime;
use zlf_prolog::{PrologParser, Query, Term};

#[test]
fn parser_distinguishes_numbers_quotes_lists_and_directives() {
    assert_eq!(PrologParser::parse_term("42").unwrap(), Term::Integer(42));
    assert_eq!(PrologParser::parse_term("4.2").unwrap(), Term::Float(4.2));
    assert_eq!(
        PrologParser::parse_term("'a,b+c'").unwrap(),
        Term::Atom("a,b+c".to_string())
    );
    assert_eq!(
        PrologParser::parse_term("'has space'(x)").unwrap(),
        compound("has space", vec![atom("x")])
    );
    assert_eq!(
        PrologParser::parse_term("[H|T]").unwrap(),
        compound(".", vec![var("H"), var("T")])
    );
    assert!(matches!(
        PrologParser::parse_query(":- dynamic likes/2.").unwrap(),
        Query::Directive(_)
    ));
}

#[test]
fn wam_builtin_executor_runs_unification_identity_and_order() {
    assert_binding("X = alice", "X", atom("alice"));
    for source in [
        "alice \\= bob",
        "alice == alice",
        "alice \\== bob",
        "alice @< bob",
        "alice @=< alice",
        "bob @> alice",
        "bob @>= bob",
    ] {
        assert_eq!(query(source).len(), 1, "{source}");
    }
    for source in ["alice \\= alice", "alice == bob", "alice \\== alice"] {
        assert!(query(source).is_empty(), "{source}");
    }
}

#[test]
fn wam_builtin_executor_runs_all_type_tests() {
    for source in [
        "var(X)",
        "nonvar(alice)",
        "atom(alice)",
        "integer(3)",
        "float(3.0)",
        "number(3)",
        "number(3.0)",
        "atomic(\"text\")",
        "compound(foo(a))",
        "compound([a])",
        "ground(foo(a, [b]))",
    ] {
        assert_eq!(query(source).len(), 1, "{source}");
    }
    for source in [
        "nonvar(X)",
        "atom(3)",
        "integer(3.0)",
        "float(3)",
        "ground(foo(X))",
    ] {
        assert!(query(source).is_empty(), "{source}");
    }
}

#[test]
fn wam_builtin_executor_decomposes_and_constructs_terms() {
    let rows = query("functor(parent(alice,bob), Name, Arity)");
    assert_eq!(rows[0]["Name"], atom("parent"));
    assert_eq!(rows[0]["Arity"], Term::Integer(2));

    let rows = query("functor(Term, parent, 2)");
    assert!(matches!(
        &rows[0]["Term"],
        Term::Compound { name, args } if name == "parent" && args.len() == 2
    ));
    assert_binding("arg(2, parent(alice,bob), X)", "X", atom("bob"));
    assert_binding(
        "parent(alice,bob) =.. L",
        "L",
        Term::List(vec![atom("parent"), atom("alice"), atom("bob")]),
    );
    assert_binding(
        "Term =.. [parent,alice,bob]",
        "Term",
        compound("parent", vec![atom("alice"), atom("bob")]),
    );
}

#[test]
fn canonical_cons_unification_uses_the_wam_unifier() {
    let rows = query("[H|T] = [a,b,c]");
    assert_eq!(rows[0]["H"], atom("a"));
    assert_eq!(rows[0]["T"], Term::List(vec![atom("b"), atom("c")]));

    let rows = query("[a,b|T] = [a,b,c,d]");
    assert_eq!(rows[0]["T"], Term::List(vec![atom("c"), atom("d")]));
    assert!(query("[a|_] = [b]").is_empty());
}

fn assert_binding(source: &str, name: &str, expected: Term) {
    let rows = query(source);
    assert_eq!(rows[0].get(name), Some(&expected), "{source}");
}

fn query(source: &str) -> Vec<HashMap<String, Term>> {
    let term = PrologParser::parse_term(source).unwrap();
    WamRuntime::new(64).query_all(&term).unwrap()
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
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
