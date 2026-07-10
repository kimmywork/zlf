use std::collections::HashMap;

use zlf_prolog::wam::WamRuntime;
use zlf_prolog::{PrologParser, Term};

#[test]
fn wam_builtin_executor_runs_all_arithmetic_predicates() {
    for source in [
        "3 =:= 1 + 2",
        "3 =\\= 4",
        "1 < 2",
        "2 =< 2",
        "3 > 2",
        "3 >= 3",
    ] {
        assert_eq!(query(source).len(), 1, "{source}");
    }
    for source in ["3 =:= 4", "3 =\\= 3", "2 < 1", "3 =< 2", "2 > 3", "2 >= 3"] {
        assert!(query(source).is_empty(), "{source}");
    }
}

#[test]
fn arithmetic_functions_preserve_integer_and_float_types() {
    let cases = [
        ("X is +3", Term::Integer(3)),
        ("X is -3", Term::Integer(-3)),
        ("X is 2 + 3", Term::Integer(5)),
        ("X is 5 - 3", Term::Integer(2)),
        ("X is 2 * 3", Term::Integer(6)),
        ("X is 5 / 2", Term::Float(2.5)),
        ("X is 5 // 2", Term::Integer(2)),
        ("X is 5 mod 2", Term::Integer(1)),
        ("X is -5 rem 2", Term::Integer(-1)),
        ("X is abs(-5)", Term::Integer(5)),
        ("X is min(2, 3)", Term::Integer(2)),
        ("X is max(2, 3)", Term::Integer(3)),
        ("X is 4 / 2", Term::Float(2.0)),
        ("X is 1 + 2.5", Term::Float(3.5)),
    ];
    for (source, expected) in cases {
        let rows = query(source);
        assert_eq!(rows[0].get("X"), Some(&expected), "{source}");
    }
}

#[test]
fn arithmetic_reports_invalid_evaluation() {
    assert!(query_result("X is Y + 1").is_err());
    assert!(query_result("X is alice + 1").is_err());
    assert!(query_result("X is 1 / 0").is_err());
    assert!(query_result("X is 1.5 // 1").is_err());
}

fn query(source: &str) -> Vec<HashMap<String, Term>> {
    query_result(source).unwrap()
}

fn query_result(source: &str) -> zlf_prolog::wam::WamResult<Vec<HashMap<String, Term>>> {
    let term = PrologParser::parse_term(source).unwrap();
    WamRuntime::new(64).query_all(&term)
}
