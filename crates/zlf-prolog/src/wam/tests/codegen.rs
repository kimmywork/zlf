use crate::PrologParser;

use super::super::{compile_query_program, WamCodegen, WamExecutor, WamProgram};

#[test]
fn codegen_runs_parent_fact_on_instruction_runtime() {
    let query = term("parent(alice, X)");
    let fact = term("parent(alice, bob)");
    let mut executor = WamExecutor::new(8);
    let program = combine(
        WamCodegen::compile_query_goal(&query).unwrap(),
        WamCodegen::compile_fact_head(&fact).unwrap(),
    );

    let result = executor.execute(&program).unwrap();

    assert!(result.success);
    assert_eq!(executor.register_term(1).unwrap(), atom("bob"));
}

#[test]
fn codegen_runs_nested_structure_fact_on_instruction_runtime() {
    let query = term("likes(alice, pair(one, Y))");
    let fact = term("likes(alice, pair(one, two))");
    let mut executor = WamExecutor::new(12);
    let program = combine(
        WamCodegen::compile_query_goal(&query).unwrap(),
        WamCodegen::compile_fact_head(&fact).unwrap(),
    );

    let result = executor.execute(&program).unwrap();

    assert!(result.success);
    assert_eq!(executor.register_term(2).unwrap(), atom("two"));
}

#[test]
fn codegen_program_collects_multi_fact_solutions() {
    let query = term("color(X)");
    let facts = vec![
        term("color(red)"),
        term("color(green)"),
        term("color(blue)"),
    ];
    let program = compile_query_program(&query, &facts, &[]).unwrap();
    let mut executor = WamExecutor::new(8);

    let solutions = executor.execute_all_registers(&program, &[0]).unwrap();

    assert_eq!(solutions.len(), 3);
    assert_eq!(solutions[0], vec![atom("red")]);
    assert_eq!(solutions[1], vec![atom("green")]);
    assert_eq!(solutions[2], vec![atom("blue")]);
}

#[test]
fn codegen_program_executes_grandparent_rule() {
    let query = term("grandparent(alice, Who)");
    let facts = vec![term("parent(alice, bob)"), term("parent(bob, carol)")];
    let rules = vec![rule("grandparent(X, Z) :- parent(X, Y), parent(Y, Z).")];
    let program = compile_query_program(&query, &facts, &rules).unwrap();
    let mut executor = WamExecutor::new(12);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(1).unwrap(), atom("carol"));
}

#[test]
fn codegen_rejects_mismatching_fact_on_instruction_runtime() {
    let query = term("parent(alice, X)");
    let fact = term("parent(carol, bob)");
    let mut executor = WamExecutor::new(8);
    let program = combine(
        WamCodegen::compile_query_goal(&query).unwrap(),
        WamCodegen::compile_fact_head(&fact).unwrap(),
    );

    assert!(!executor.execute(&program).unwrap().success);
}

fn combine(query: WamProgram, fact: WamProgram) -> WamProgram {
    let mut instructions = query.instructions().to_vec();
    instructions.extend_from_slice(fact.instructions());
    WamProgram::new(instructions)
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
