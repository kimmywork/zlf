use zlf_prolog::PrologParser;
use zlf_query::ZlfDatabase;

#[test]
fn table_directive_terminates_recursive_rules_over_a_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let db = cyclic_database(dir.path());

    let rows = db.query_prolog("? path(a, X).").unwrap();
    let mut values = rows
        .iter()
        .map(|row| row["X"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    values.sort();
    assert_eq!(values, vec!["a", "b", "c", "d"]);

    let builtin = builtin_reachable(&db);
    assert_eq!(
        values
            .iter()
            .filter(|value| value.as_str() != "a")
            .cloned()
            .collect::<Vec<_>>(),
        builtin
    );
}

fn cyclic_database(path: &std::path::Path) -> ZlfDatabase {
    let db = ZlfDatabase::open(path).unwrap();
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
    db.query_prolog("path(X,Y) :- path(X,Z), knows(Z,Y).")
        .unwrap();
    db.query_prolog(":- table path/2.").unwrap();
    db
}

fn builtin_reachable(db: &ZlfDatabase) -> Vec<String> {
    let mut values = db
        .query_prolog("? reachable(a, X).")
        .unwrap()
        .iter()
        .map(|row| row["X"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    values.sort();
    values
}

#[test]
fn fact_mutation_invalidates_persistent_answers_before_recompute() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    for fact in ["edge(a, next, b).", "edge(b, next, c)."] {
        db.apply_fact(&PrologParser::parse_fact(fact).unwrap().head)
            .unwrap();
    }
    db.query_prolog("path(X,Y) :- next(X,Y).").unwrap();
    db.query_prolog("path(X,Y) :- path(X,Z), next(Z,Y).")
        .unwrap();
    db.query_prolog(":- table path/2.").unwrap();
    assert_eq!(db.query_prolog("? path(a,X).").unwrap().len(), 2);

    db.apply_fact(&PrologParser::parse_fact("edge(c, next, d).").unwrap().head)
        .unwrap();
    assert_eq!(db.query_prolog("? path(a,X).").unwrap().len(), 3);
    assert!(db.table_metrics().stale_invalidations > 0);
}

#[test]
fn declarations_and_complete_answers_survive_database_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = ZlfDatabase::open(dir.path()).unwrap();
        for fact in ["node(a).", "node(b).", "edge(a, next, b)."] {
            db.apply_fact(&PrologParser::parse_fact(fact).unwrap().head)
                .unwrap();
        }
        db.query_prolog("path(X,Y) :- next(X,Y).").unwrap();
        db.query_prolog(":- table path/2.").unwrap();
        assert_eq!(db.query_prolog("? path(a,X).").unwrap().len(), 1);
    }
    let reopened = ZlfDatabase::open_existing(dir.path()).unwrap();
    assert_eq!(reopened.query_prolog("? path(a,X).").unwrap().len(), 1);
    assert_eq!(reopened.table_metrics().persistent_hits, 1);
}
