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
fn predicate_dependencies_preserve_unrelated_complete_tables() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    for fact in ["edge(a, next, b).", "edge(x, other, y)."] {
        db.apply_fact(&PrologParser::parse_fact(fact).unwrap().head)
            .unwrap();
    }
    for rule in [
        "path(X,Y) :- next(X,Y).",
        "path(X,Y) :- path(X,Z), next(Z,Y).",
        "other_path(X,Y) :- other(X,Y).",
    ] {
        db.query_prolog(rule).unwrap();
    }
    for directive in [":- table path/2.", ":- table other_path/2."] {
        db.query_prolog(directive).unwrap();
    }
    db.query_prolog("? path(a,X).").unwrap();
    db.query_prolog("? other_path(x,X).").unwrap();
    assert_eq!(db.table_metrics().tables_completed, 2);

    db.apply_fact(&PrologParser::parse_fact("edge(b, next, c).").unwrap().head)
        .unwrap();
    assert_eq!(db.query_prolog("? other_path(x,X).").unwrap().len(), 1);
    assert_eq!(db.table_metrics().tables_completed, 2);
    assert_eq!(db.query_prolog("? path(a,X).").unwrap().len(), 2);
    assert_eq!(db.table_metrics().tables_completed, 3);
}

#[test]
fn dynamic_retract_selectively_invalidates_its_predicate_tables() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    for fact in ["edge(a, next, b).", "edge(x, other, y)."] {
        db.apply_fact(&PrologParser::parse_fact(fact).unwrap().head)
            .unwrap();
    }
    for rule in ["path(X,Y) :- next(X,Y).", "other_path(X,Y) :- other(X,Y)."] {
        db.query_prolog(rule).unwrap();
    }
    for directive in [":- table path/2.", ":- table other_path/2."] {
        db.query_prolog(directive).unwrap();
    }
    db.query_prolog("? path(a,X).").unwrap();
    db.query_prolog("? other_path(x,X).").unwrap();
    db.query_prolog("? retract(next(a,b)).").unwrap();

    assert_eq!(db.query_prolog("? other_path(x,X).").unwrap().len(), 1);
    assert_eq!(db.table_metrics().tables_completed, 2);
    assert!(db.query_prolog("? path(a,X).").unwrap().is_empty());
    assert_eq!(db.table_metrics().tables_completed, 3);
}

#[test]
fn exact_retract_preserves_unrelated_variants_of_the_same_table() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    for fact in ["edge(a, next, b).", "edge(x, next, y)."] {
        db.apply_fact(&PrologParser::parse_fact(fact).unwrap().head)
            .unwrap();
    }
    db.query_prolog("path(X,Y) :- next(X,Y).").unwrap();
    db.query_prolog(":- table path/2.").unwrap();
    db.query_prolog("? path(a,X).").unwrap();
    db.query_prolog("? path(x,X).").unwrap();
    assert_eq!(db.table_metrics().tables_completed, 2);

    db.query_prolog("? retract(next(a,b)).").unwrap();
    assert_eq!(db.query_prolog("? path(x,X).").unwrap().len(), 1);
    assert_eq!(db.table_metrics().tables_completed, 2);
    assert!(db.query_prolog("? path(a,X).").unwrap().is_empty());
    assert_eq!(db.table_metrics().tables_completed, 3);
}

#[test]
fn exact_fact_invalidation_propagates_to_dependent_tables() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    for fact in ["edge(a, next, c).", "edge(x, next, c)."] {
        db.apply_fact(&PrologParser::parse_fact(fact).unwrap().head)
            .unwrap();
    }
    for rule in [
        "path(X,Y) :- next(X,Y).",
        "common(A,B,Y) :- path(A,Y), path(B,Y).",
    ] {
        db.query_prolog(rule).unwrap();
    }
    for directive in [":- table path/2.", ":- table common/3."] {
        db.query_prolog(directive).unwrap();
    }
    assert_eq!(db.query_prolog("? common(a,x,Y).").unwrap().len(), 1);
    let completed = db.table_metrics().tables_completed;

    db.query_prolog("? retract(next(a,c)).").unwrap();
    assert_eq!(db.query_prolog("? path(x,Y).").unwrap().len(), 1);
    assert_eq!(db.table_metrics().tables_completed, completed);
    assert!(db.query_prolog("? common(a,x,Y).").unwrap().is_empty());
    assert!(db.table_metrics().tables_completed > completed);
}

#[test]
fn node_retract_invalidates_tables_for_cascaded_relations() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    for fact in [
        "node(a, [taxon], {}).",
        "node(b, [taxon], {}).",
        "edge(a, next, b).",
    ] {
        db.apply_fact(&PrologParser::parse_fact(fact).unwrap().head)
            .unwrap();
    }
    for rule in ["tagged(X) :- taxon(X).", "linked(X,Y) :- next(X,Y)."] {
        db.query_prolog(rule).unwrap();
    }
    for directive in [":- table tagged/1.", ":- table linked/2."] {
        db.query_prolog(directive).unwrap();
    }
    assert_eq!(db.query_prolog("? linked(a,Y).").unwrap().len(), 1);
    assert_eq!(db.query_prolog("? tagged(a).").unwrap().len(), 1);

    db.query_prolog("? retract(node(a)).").unwrap();
    assert!(db.query_prolog("? linked(a,Y).").unwrap().is_empty());
    assert!(db.query_prolog("? tagged(a).").unwrap().is_empty());
}

#[test]
fn selective_stale_state_persists_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = ZlfDatabase::open(dir.path()).unwrap();
        for fact in ["edge(a, next, b).", "edge(x, other, y)."] {
            db.apply_fact(&PrologParser::parse_fact(fact).unwrap().head)
                .unwrap();
        }
        db.query_prolog("path(X,Y) :- next(X,Y).").unwrap();
        db.query_prolog("other_path(X,Y) :- other(X,Y).").unwrap();
        db.query_prolog(":- table path/2.").unwrap();
        db.query_prolog(":- table other_path/2.").unwrap();
        db.query_prolog("? path(a,X).").unwrap();
        db.query_prolog("? other_path(x,X).").unwrap();
    }
    {
        let db = ZlfDatabase::open_existing(dir.path()).unwrap();
        db.apply_fact(&PrologParser::parse_fact("edge(b, next, c).").unwrap().head)
            .unwrap();
    }
    let reopened = ZlfDatabase::open_existing(dir.path()).unwrap();
    reopened.query_prolog("? other_path(x,X).").unwrap();
    assert_eq!(reopened.table_metrics().persistent_hits, 1);
    reopened.query_prolog("? path(a,X).").unwrap();
    assert_eq!(reopened.table_metrics().tables_completed, 1);
}

#[test]
fn explicit_property_mutation_invalidates_exact_tabled_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    db.apply_fact(
        &PrologParser::parse_fact("node(alice, [], {name: \"old\"}).")
            .unwrap()
            .head,
    )
    .unwrap();
    db.query_prolog("named(X,N) :- prop_name(X,N).").unwrap();
    db.query_prolog(":- table named/2.").unwrap();
    assert_eq!(db.query_prolog("? named(alice,N).").unwrap()[0]["N"], "old");
    db.query_prolog("? set_node_property(alice, name, \"new\").")
        .unwrap();
    assert_eq!(db.query_prolog("? named(alice,N).").unwrap()[0]["N"], "new");
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
