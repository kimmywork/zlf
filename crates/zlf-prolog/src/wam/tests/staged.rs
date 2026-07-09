use crate::PrologParser;

use super::super::{
    compile_fact, compile_facts, compile_rule, permanent_variables, EnvironmentFrame, M1Machine,
    M2Machine, M3Machine, M3Program,
};

#[test]
fn m1_matches_fact_call_and_returns_binding() {
    let goal = PrologParser::parse_term("parent(alice, X)").unwrap();
    let fact = compile_fact(PrologParser::parse_term("parent(alice, bob)").unwrap()).unwrap();
    let mut machine = M1Machine::new(8);

    let solution = machine.solve_fact(&goal, &fact).unwrap().unwrap();

    assert_eq!(solution.get("X"), Some(&atom("bob")));
}

#[test]
fn m1_rejects_non_matching_fact_call() {
    let goal = PrologParser::parse_term("parent(alice, X)").unwrap();
    let fact = compile_fact(PrologParser::parse_term("parent(carol, bob)").unwrap()).unwrap();
    let mut machine = M1Machine::new(8);

    assert!(machine.solve_fact(&goal, &fact).unwrap().is_none());
}

#[test]
fn m2_classifies_permanent_variables() {
    let rule =
        PrologParser::parse_rule("grandparent(X, Z) :- parent(X, Y), parent(Y, Z).").unwrap();

    assert_eq!(permanent_variables(&rule), vec!["Y".to_string()]);
}

#[test]
fn m2_environment_frame_stores_permanent_variables() {
    let vars = vec!["Y".to_string()];
    let mut frame = EnvironmentFrame::allocate(&vars, Some(10), Some(1));

    assert_eq!(frame.slot_count(), 1);
    assert!(frame.set("Y", atom("bob")));
    assert_eq!(frame.get("Y"), Some(&atom("bob")));
    assert_eq!(frame.continuation(), Some(10));
    assert_eq!(frame.previous(), Some(1));
}

#[test]
fn m2_executes_two_goal_rule_body() {
    let facts =
        compile_facts(vec![term("parent(alice, bob)"), term("parent(bob, carol)")]).unwrap();
    let rule = compile_rule(rule("grandparent(X, Z) :- parent(X, Y), parent(Y, Z).")).unwrap();
    assert_eq!(rule.permanent_vars, vec!["Y".to_string()]);
    let machine = M2Machine::new(8);

    let solutions = machine
        .solve_rule(&term("grandparent(alice, Who)"), &rule, &facts)
        .unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get("Who"), Some(&atom("carol")));
}

#[test]
fn m2_returns_empty_when_rule_body_fails() {
    let facts = compile_facts(vec![term("parent(alice, bob)")]).unwrap();
    let rule = compile_rule(rule("grandparent(X, Z) :- parent(X, Y), parent(Y, Z).")).unwrap();
    let machine = M2Machine::new(8);

    assert!(machine
        .solve_rule(&term("grandparent(alice, Who)"), &rule, &facts)
        .unwrap()
        .is_empty());
}

#[test]
fn m3_backtracks_over_multiple_fact_alternatives() {
    let mut program = M3Program::new();
    program.add_fact(term("color(red)")).unwrap();
    program.add_fact(term("color(green)")).unwrap();
    program.add_fact(term("color(blue)")).unwrap();
    let machine = M3Machine::new(8);

    let solutions = machine.solve(&term("color(X)"), &program).unwrap();

    assert_eq!(solutions.len(), 3);
    assert_eq!(solutions[0].get("X"), Some(&atom("red")));
    assert_eq!(solutions[1].get("X"), Some(&atom("green")));
    assert_eq!(solutions[2].get("X"), Some(&atom("blue")));
}

#[test]
fn m3_backtracks_through_rule_body_alternatives() {
    let mut program = M3Program::new();
    program.add_fact(term("parent(alice, bob)")).unwrap();
    program.add_fact(term("parent(alice, beth)")).unwrap();
    program
        .add_rule(rule("child_of_alice(X) :- parent(alice, X)."))
        .unwrap();
    let machine = M3Machine::new(8);

    let solutions = machine
        .solve(&term("child_of_alice(Who)"), &program)
        .unwrap();

    assert_eq!(solutions.len(), 2);
    assert_eq!(solutions[0].get("Who"), Some(&atom("bob")));
    assert_eq!(solutions[1].get("Who"), Some(&atom("beth")));
}

fn atom(value: &str) -> crate::Term {
    crate::Term::Atom(value.to_string())
}

fn term(source: &str) -> crate::Term {
    PrologParser::parse_term(source).unwrap()
}

fn rule(source: &str) -> crate::parser::PrologRule {
    PrologParser::parse_rule(source).unwrap()
}
