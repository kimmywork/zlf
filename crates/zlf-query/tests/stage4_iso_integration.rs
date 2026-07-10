use zlf_query::ZlfDatabase;

#[test]
fn dynamic_rule_assert_clause_query_and_retract_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    db.query_prolog("knows(alice, bob).").unwrap();

    assert_eq!(
        db.query_prolog("? assertz((friend(X,Y) :- knows(X,Y))).")
            .unwrap()
            .len(),
        1
    );
    let friends = db.query_prolog("? friend(alice, X).").unwrap();
    assert_eq!(friends.len(), 1);
    assert_eq!(friends[0]["X"], "bob");

    let clauses = db.query_prolog("? clause(friend(X,Y), Body).").unwrap();
    assert_eq!(clauses.len(), 1);
    assert_eq!(clauses[0]["Body"]["name"], "knows");

    assert_eq!(
        db.query_prolog("? retract((friend(X,Y) :- knows(X,Y))).")
            .unwrap()
            .len(),
        1
    );
    assert!(db.query_prolog("? friend(alice, X).").unwrap().is_empty());
}

#[test]
fn directives_and_current_predicate_enumeration_are_available() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    assert!(db.query_prolog(":- dynamic likes/2.").unwrap().is_empty());
    db.query_prolog("? assertz(likes(alice, tea)).").unwrap();

    let predicates = db.query_prolog("? current_predicate(P).").unwrap();
    assert!(predicates.iter().any(|row| {
        row["P"]["name"] == "/" && row["P"]["args"] == serde_json::json!(["likes", 2])
    }));
}

#[test]
fn query_facade_exposes_opt_in_proof_answers() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    db.apply_fact(
        &zlf_prolog::PrologParser::parse_fact("parent(alice, bob).")
            .unwrap()
            .head,
    )
    .unwrap();
    db.query_prolog("ancestor(X,Y) :- parent(X,Y).").unwrap();

    let answers = db.query_prolog_with_proof("? ancestor(alice, X).").unwrap();
    assert_eq!(
        answers[0].bindings["X"],
        zlf_prolog::Term::Atom("bob".into())
    );
    assert!(answers[0].proof.nodes.iter().any(|node| {
        node.clause.predicate.name == "ancestor" && !node.substitutions.is_empty()
    }));
    assert!(answers[0]
        .proof
        .nodes
        .iter()
        .any(|node| node.clause.kind == zlf_prolog::wam::ProofKind::Fact));
}
