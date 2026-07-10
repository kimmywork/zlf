use zlf_prolog::wam::{ProofKind, WamRuntime};
use zlf_prolog::{PrologParser, Term};

#[test]
fn proof_capture_is_opt_in_and_records_fact_and_rule_nodes() {
    let runtime = family_runtime();
    let query = term("grandparent(alice, Who)");

    let ordinary = runtime.query_all(&query).unwrap();
    assert_eq!(ordinary[0]["Who"], atom("carol"));

    let answers = runtime.query_all_with_proof(&query).unwrap();
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].bindings["Who"], atom("carol"));
    assert!(answers[0]
        .proof
        .nodes
        .iter()
        .any(|node| node.clause.kind == ProofKind::Rule
            && node.clause.predicate.name == "grandparent"));
    assert!(answers[0]
        .proof
        .nodes
        .iter()
        .any(|node| node.clause.kind == ProofKind::Fact && node.clause.predicate.name == "parent"));
    assert!(answers[0]
        .proof
        .nodes
        .iter()
        .any(|node| node.parent.is_some()));
    assert!(answers[0]
        .proof
        .nodes
        .iter()
        .any(|node| node.substitutions.iter().any(|term| term == &atom("alice"))));
}

#[test]
fn backtracking_rolls_proof_nodes_back_to_the_choice_point() {
    let mut runtime = WamRuntime::new(32);
    runtime.add_fact(term("color(red)"));
    runtime.add_fact(term("color(green)"));

    let answers = runtime.query_all_with_proof(&term("color(X)")).unwrap();
    assert_eq!(answers.len(), 2);
    for answer in answers {
        assert_eq!(answer.proof.nodes.len(), 1);
        assert_eq!(answer.proof.nodes[0].clause.kind, ProofKind::Fact);
    }
}

#[test]
fn successful_builtins_are_proof_leaves_with_substitutions() {
    let answers = WamRuntime::new(16)
        .query_all_with_proof(&term("X is 1 + 2"))
        .unwrap();
    let builtin = answers[0]
        .proof
        .nodes
        .iter()
        .find(|node| node.clause.kind == ProofKind::Builtin)
        .unwrap();
    assert_eq!(builtin.clause.predicate.name, "is");
    assert_eq!(builtin.substitutions[0], Term::Integer(3));
}

#[test]
fn clause_ids_are_stable_for_identical_sources() {
    let first = family_runtime()
        .query_all_with_proof(&term("grandparent(alice, X)"))
        .unwrap();
    let second = family_runtime()
        .query_all_with_proof(&term("grandparent(alice, X)"))
        .unwrap();
    let first_ids: Vec<_> = first[0]
        .proof
        .nodes
        .iter()
        .map(|node| node.clause.id.clone())
        .collect();
    let second_ids: Vec<_> = second[0]
        .proof
        .nodes
        .iter()
        .map(|node| node.clause.id.clone())
        .collect();
    assert_eq!(first_ids, second_ids);
}

fn family_runtime() -> WamRuntime {
    let mut runtime = WamRuntime::new(64);
    runtime.add_fact(term("parent(alice, bob)"));
    runtime.add_fact(term("parent(bob, carol)"));
    runtime.add_rule(
        PrologParser::parse_rule("grandparent(X,Z) :- parent(X,Y), parent(Y,Z).").unwrap(),
    );
    runtime
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}
