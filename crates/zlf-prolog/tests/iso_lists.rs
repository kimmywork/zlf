use std::collections::HashMap;

use zlf_prolog::wam::WamRuntime;
use zlf_prolog::{PrologParser, Term};

#[test]
fn wam_library_rules_enumerate_member_append_and_select() {
    assert_eq!(bindings("member(X, [a,b,c])", "X"), atoms(&["a", "b", "c"]));

    let rows = query("append(X, Y, [a,b])");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["X"], Term::List(vec![]));
    assert_eq!(rows[0]["Y"], list(&["a", "b"]));
    assert_eq!(rows[1]["X"], list(&["a"]));
    assert_eq!(rows[1]["Y"], list(&["b"]));
    assert_eq!(rows[2]["X"], list(&["a", "b"]));
    assert_eq!(rows[2]["Y"], Term::List(vec![]));

    let rows = query("select(X, [a,b,a], Rest)");
    assert_eq!(bindings_from(&rows, "X"), atoms(&["a", "b", "a"]));
    assert_eq!(rows[0]["Rest"], list(&["b", "a"]));
    assert_eq!(rows[1]["Rest"], list(&["a", "a"]));
}

#[test]
fn wam_builtin_executor_runs_length_reverse_and_indexing() {
    assert_eq!(bindings("length([a,b,c], N)", "N"), vec![Term::Integer(3)]);
    let rows = query("length(List, 3)");
    assert!(matches!(&rows[0]["List"], Term::List(items) if items.len() == 3));

    assert_eq!(
        bindings("reverse([a,b,c], R)", "R"),
        vec![list(&["c", "b", "a"])]
    );
    assert_eq!(bindings("nth0(1, [a,b,c], X)", "X"), atoms(&["b"]));
    assert_eq!(bindings("nth1(2, [a,b,c], X)", "X"), atoms(&["b"]));
    assert!(query("nth0(3, [a,b,c], X)").is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn wam_builtin_executor_runs_all_text_conversion_modes() {
    let cases = [
        (
            "atom_string(alice, X)",
            "X",
            Term::String("alice".to_string()),
        ),
        ("atom_string(X, \"alice\")", "X", atom("alice")),
        ("atom_chars(ab, X)", "X", list(&["a", "b"])),
        ("atom_chars(X, [a,b])", "X", atom("ab")),
        ("string_chars(\"ab\", X)", "X", list(&["a", "b"])),
        (
            "string_chars(X, [a,b])",
            "X",
            Term::String("ab".to_string()),
        ),
        (
            "atom_codes(ab, X)",
            "X",
            Term::List(vec![Term::Integer(97), Term::Integer(98)]),
        ),
        ("atom_codes(X, [97,98])", "X", atom("ab")),
        (
            "string_codes(\"ab\", X)",
            "X",
            Term::List(vec![Term::Integer(97), Term::Integer(98)]),
        ),
        (
            "string_codes(X, [97,98])",
            "X",
            Term::String("ab".to_string()),
        ),
        ("number_string(42, X)", "X", Term::String("42".to_string())),
        ("number_string(X, \"42.5\")", "X", Term::Float(42.5)),
    ];
    for (source, binding, expected) in cases {
        let rows = query(source);
        assert_eq!(rows[0].get(binding), Some(&expected), "{source}");
    }
}

fn bindings(source: &str, name: &str) -> Vec<Term> {
    bindings_from(&query(source), name)
}

fn bindings_from(rows: &[HashMap<String, Term>], name: &str) -> Vec<Term> {
    rows.iter().map(|row| row[name].clone()).collect()
}

fn query(source: &str) -> Vec<HashMap<String, Term>> {
    let term = PrologParser::parse_term(source).unwrap();
    WamRuntime::new(64).query_all(&term).unwrap()
}

fn atoms(values: &[&str]) -> Vec<Term> {
    values.iter().map(|value| atom(value)).collect()
}

fn list(values: &[&str]) -> Term {
    Term::List(atoms(values))
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}
