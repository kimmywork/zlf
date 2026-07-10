use crate::parser::Term;
use crate::wam::{CompositeFactProvider, StorageFactProvider, StorageFactWriter, WamRuntime};
use zlf_storage::Storage;

#[test]
#[allow(clippy::too_many_lines)]
fn wam_executes_pure_builtin_executor_predicates() {
    let cases = [
        (
            compound(
                "is",
                vec![var("X"), compound("+", vec![num(1.0), num(2.0)])],
            ),
            "X",
            Term::Integer(3),
        ),
        (
            compound(
                "=:=",
                vec![num(3.0), compound("+", vec![num(1.0), num(2.0)])],
            ),
            "",
            atom("ok"),
        ),
        (
            compound(
                "functor",
                vec![
                    compound("foo", vec![atom("a"), atom("b")]),
                    var("F"),
                    var("A"),
                ],
            ),
            "F",
            atom("foo"),
        ),
        (
            compound(
                "arg",
                vec![
                    num(2.0),
                    compound("foo", vec![atom("a"), atom("b")]),
                    var("X"),
                ],
            ),
            "X",
            atom("b"),
        ),
        (
            compound(
                "=..",
                vec![compound("foo", vec![atom("a"), atom("b")]), var("X")],
            ),
            "X",
            list(vec![atom("foo"), atom("a"), atom("b")]),
        ),
        (
            compound("member", vec![var("X"), list(vec![atom("a"), atom("b")])]),
            "X",
            atom("a"),
        ),
        (
            compound(
                "append",
                vec![list(vec![atom("a")]), list(vec![atom("b")]), var("X")],
            ),
            "X",
            list(vec![atom("a"), atom("b")]),
        ),
        (
            compound(
                "nth0",
                vec![num(1.0), list(vec![atom("a"), atom("b")]), var("X")],
            ),
            "X",
            atom("b"),
        ),
        (
            compound("string_chars", vec![Term::String("ab".into()), var("X")]),
            "X",
            list(vec![atom("a"), atom("b")]),
        ),
        (
            compound("number_string", vec![var("X"), Term::String("42".into())]),
            "X",
            Term::Integer(42),
        ),
        (
            compound("=", vec![var("X"), atom("alice")]),
            "X",
            atom("alice"),
        ),
    ];
    for (term, binding, expected) in cases {
        let rows = query(term.clone());
        assert!(!rows.is_empty(), "{term:?} should succeed");
        if !binding.is_empty() {
            assert_eq!(rows[0].get(binding), Some(&expected), "{term:?}");
        }
    }
    assert!(query(compound("\\==", vec![atom("alice"), atom("bob")])).len() == 1);
    assert!(query(compound("@<", vec![atom("alice"), atom("bob")])).len() == 1);
    assert!(query(atom("fail")).is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn wam_executes_control_and_dynamic_builtin_executor_predicates() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("storage")).unwrap();
    let writer = StorageFactWriter::new(&storage);
    writer
        .apply_fact(&compound("likes", vec![atom("alice"), atom("bob")]))
        .unwrap();
    writer
        .apply_fact(&compound("likes", vec![atom("alice"), atom("carol")]))
        .unwrap();

    let provider = StorageFactProvider::new(&storage);
    let provider = CompositeFactProvider::new().with(&provider);
    let runtime = WamRuntime::new(64);

    let rows = runtime
        .query_all_with_provider_and_storage(
            &compound("call", vec![atom("likes"), atom("alice"), var("X")]),
            &provider,
            &storage,
        )
        .unwrap();
    assert_eq!(rows.len(), 2);

    let rows = runtime
        .query_all_with_provider_and_storage(
            &compound(
                "once",
                vec![compound("likes", vec![atom("alice"), var("X")])],
            ),
            &provider,
            &storage,
        )
        .unwrap();
    assert_eq!(rows.len(), 1);

    assert_eq!(
        runtime
            .query_all_with_provider_and_storage(
                &compound(
                    "\\+",
                    vec![compound("likes", vec![atom("alice"), atom("dave")])]
                ),
                &provider,
                &storage
            )
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        runtime
            .query_all_with_provider_and_storage(
                &compound(
                    ";",
                    vec![
                        compound("likes", vec![atom("alice"), atom("bob")]),
                        compound("likes", vec![atom("alice"), atom("dave")])
                    ]
                ),
                &provider,
                &storage
            )
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        runtime
            .query_all_with_provider_and_storage(
                &compound(
                    "assertz",
                    vec![compound("likes", vec![atom("alice"), atom("dave")])]
                ),
                &provider,
                &storage
            )
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        runtime
            .query_all_with_provider_and_storage(
                &compound(
                    "retract",
                    vec![compound("likes", vec![atom("alice"), atom("bob")])]
                ),
                &provider,
                &storage
            )
            .unwrap()
            .len(),
        1
    );
    assert!(runtime
        .query_all_with_provider_and_storage(
            &compound("likes", vec![atom("alice"), atom("bob")]),
            &provider,
            &storage,
        )
        .unwrap()
        .is_empty());
    assert_eq!(
        runtime
            .query_all_with_provider_and_storage(
                &compound("current_predicate", vec![Term::String("likes/2".into())]),
                &provider,
                &storage
            )
            .unwrap()
            .len(),
        1
    );
}

fn query(term: Term) -> Vec<std::collections::HashMap<String, Term>> {
    WamRuntime::new(64).query_all(&term).unwrap()
}

fn var(name: &str) -> Term {
    Term::Variable(name.to_string())
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}

fn num(value: f64) -> Term {
    if value.fract() == 0.0 {
        Term::Integer(value as i64)
    } else {
        Term::Float(value)
    }
}

fn list(items: Vec<Term>) -> Term {
    Term::List(items)
}

fn compound(name: &str, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.to_string(),
        args,
    }
}
