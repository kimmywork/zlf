use zlf_prolog::PrologParser;
use zlf_query::ZlfDatabase;

#[test]
fn table_directive_terminates_recursive_rules_over_a_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    for edge in [
        "edge(a, knows, b).",
        "edge(b, knows, c).",
        "edge(c, knows, a).",
        "edge(c, knows, d).",
    ] {
        db.apply_fact(&PrologParser::parse_fact(edge).unwrap().head)
            .unwrap();
    }
    db.query_prolog("path(X,Y) :- knows(X,Y).").unwrap();
    db.query_prolog("path(X,Y) :- knows(X,Z), path(Z,Y).")
        .unwrap();
    db.query_prolog(":- table path/2.").unwrap();

    let rows = db.query_prolog("? path(a, X).").unwrap();
    let mut values = rows
        .iter()
        .map(|row| row["X"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    values.sort();
    assert_eq!(values, vec!["a", "b", "c", "d"]);

    let mut builtin = db
        .query_prolog("? reachable(a, X).")
        .unwrap()
        .iter()
        .map(|row| row["X"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    builtin.sort();
    assert_eq!(values, builtin);
}
