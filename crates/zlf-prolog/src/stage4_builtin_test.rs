use crate::parser::Term;
use crate::parser_expr::parse_term_expr;
use crate::wam::{FactProvider, GraphAlgorithmProvider, WamRuntime};
use zlf_core::{Edge, Node};
use zlf_storage::Storage;

#[test]
fn parser_expr_parses_stage4_control_and_comparison_operators() {
    assert_eq!(
        parse_term_expr("alice == bob").unwrap(),
        compound("==", vec![atom("alice"), atom("bob")])
    );
    assert_eq!(
        parse_term_expr("\\+ knows(alice,bob)").unwrap(),
        compound(
            "\\+",
            vec![compound("knows", vec![atom("alice"), atom("bob")])],
        )
    );
    assert_eq!(
        parse_term_expr("member(X,[a]); member(X,[b])").unwrap(),
        compound(
            ";",
            vec![
                compound("member", vec![var("X"), Term::List(vec![atom("a")])]),
                compound("member", vec![var("X"), Term::List(vec![atom("b")])]),
            ],
        )
    );
    assert_eq!(
        parse_term_expr("current_predicate(likes/2)").unwrap(),
        compound(
            "current_predicate",
            vec![compound("/", vec![atom("likes"), Term::Integer(2)])],
        )
    );
}

#[test]
fn wam_executes_member_append_and_nth_builtins() {
    let runtime = WamRuntime::new(32);
    let list = Term::List(vec![atom("a"), atom("b"), atom("c")]);
    let member = runtime
        .query_all(&compound("member", vec![var("X"), list.clone()]))
        .unwrap();
    assert_eq!(member.len(), 3);
    assert_eq!(member[1]["X"], atom("b"));

    let append = runtime
        .query_all(&compound(
            "append",
            vec![
                Term::List(vec![atom("a"), atom("b")]),
                Term::List(vec![atom("c")]),
                var("X"),
            ],
        ))
        .unwrap();
    assert_eq!(append[0]["X"], list.clone());

    let nth = runtime
        .query_all(&compound("nth0", vec![Term::Integer(1), list, var("X")]))
        .unwrap();
    assert_eq!(nth[0]["X"], atom("b"));
}

#[test]
fn wam_executes_string_chars_and_number_string_builtins() {
    let runtime = WamRuntime::new(32);
    let chars = runtime
        .query_all(&compound(
            "string_chars",
            vec![Term::String("ab".into()), var("Cs")],
        ))
        .unwrap();
    assert_eq!(chars[0]["Cs"], Term::List(vec![atom("a"), atom("b")]));

    let number = runtime
        .query_all(&compound(
            "number_string",
            vec![var("N"), Term::String("42".into())],
        ))
        .unwrap();
    assert_eq!(number[0]["N"], Term::Integer(42));
}

#[test]
#[allow(clippy::too_many_lines)]
fn graph_algorithm_provider_enumerates_shortest_paths_to_variable_targets() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("storage")).unwrap();
    for id in ["a", "b", "c"] {
        storage
            .create_node(Node::with_id(
                id.to_string(),
                Vec::new(),
                Default::default(),
            ))
            .unwrap();
    }
    storage
        .create_edge(Edge::new(
            "follows".to_string(),
            "b".to_string(),
            "a".to_string(),
            Default::default(),
        ))
        .unwrap();
    storage
        .create_edge(Edge::new(
            "follows".to_string(),
            "c".to_string(),
            "b".to_string(),
            Default::default(),
        ))
        .unwrap();

    let provider = GraphAlgorithmProvider::new(&storage);
    let facts = provider
        .facts_for_goal(&compound(
            "shortest_path",
            vec![atom("c"), var("X"), var("P")],
        ))
        .unwrap();
    assert_eq!(facts.len(), 2);
    assert_eq!(
        facts[0],
        compound(
            "shortest_path",
            vec![atom("c"), atom("b"), Term::List(vec![atom("c"), atom("b")])],
        )
    );
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}

fn var(name: &str) -> Term {
    Term::Variable(name.to_string())
}

fn compound(name: &str, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.to_string(),
        args,
    }
}
