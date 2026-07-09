use crate::PrologParser;

use super::super::environment_stack::EnvironmentStack;
use super::super::{Cell, ChoicePointFrame, M0Compiler, M0Machine, RegisterFile};

#[test]
fn unifies_variable_with_constant() {
    let mut machine = M0Machine::new();
    let x = machine.set_variable();
    let atom = machine.put_constant("alice");

    assert!(machine.unify(x, atom).unwrap());
    let bound = machine.heap().deref(x).unwrap();
    assert!(matches!(machine.heap().cell(bound).unwrap(), Cell::Constant(v) if v == "alice"));
}

#[test]
fn rejects_different_constants() {
    let mut machine = M0Machine::new();
    let alice = machine.put_constant("alice");
    let bob = machine.put_constant("bob");

    assert!(!machine.unify(alice, bob).unwrap());
}

#[test]
fn failed_unification_unwinds_trail() {
    let mut machine = M0Machine::new();
    let left = machine.put_structure("pair", 2);
    let z = machine.set_variable();
    machine.set_value(z).unwrap();
    let right = machine.put_structure("pair", 2);
    machine.put_constant("one");
    machine.put_constant("two");

    assert!(!machine.unify(left, right).unwrap());
    assert!(machine.heap().is_unbound_ref(z).unwrap());
}

#[test]
fn explicit_trail_checkpoint_can_be_unwound() {
    let mut machine = M0Machine::new();
    let checkpoint = machine.trail_checkpoint();
    let x = machine.set_variable();
    let alice = machine.put_constant("alice");

    assert!(machine.unify(x, alice).unwrap());
    machine.unwind_trail(checkpoint).unwrap();

    assert!(machine.heap().is_unbound_ref(x).unwrap());
}

#[test]
fn choice_point_frame_restores_heap_trail_and_registers() {
    let mut machine = M0Machine::new();
    let mut registers = RegisterFile::new(2);
    let x = machine.set_variable();
    registers.set(0, x).unwrap();
    let mut environments = EnvironmentStack::new();
    let frame = ChoicePointFrame::capture(&machine, &registers, &environments, Some(42), 1);

    let alice = machine.put_constant("alice");
    assert!(machine.unify(x, alice).unwrap());
    registers.set(0, alice).unwrap();
    frame
        .restore(&mut machine, &mut registers, &mut environments)
        .unwrap();

    assert!(machine.heap().is_unbound_ref(x).unwrap());
    assert_eq!(registers.get(0).unwrap(), x);
    assert_eq!(frame.continuation(), Some(42));
    assert_eq!(frame.next_alternative(), 1);
}

#[test]
fn unifies_matching_structures() {
    let mut machine = M0Machine::new();
    let left = machine.put_structure("parent", 2);
    machine.put_constant("alice");
    let x = machine.set_variable();

    let right = machine.put_structure("parent", 2);
    machine.put_constant("alice");
    machine.put_constant("bob");

    assert!(machine.unify(left, right).unwrap());
    let value = machine.heap().deref(x).unwrap();
    assert!(matches!(machine.heap().cell(value).unwrap(), Cell::Constant(v) if v == "bob"));
}

#[test]
fn preserves_shared_variables() {
    let mut machine = M0Machine::new();
    let left = machine.put_structure("pair", 2);
    let z = machine.set_variable();
    machine.set_value(z).unwrap();

    let right = machine.put_structure("pair", 2);
    machine.put_constant("same");
    machine.put_constant("same");

    assert!(machine.unify(left, right).unwrap());
    let value = machine.heap().deref(z).unwrap();
    assert!(matches!(machine.heap().cell(value).unwrap(), Cell::Constant(v) if v == "same"));
}

#[test]
fn rejects_shared_variable_conflict() {
    let mut machine = M0Machine::new();
    let left = machine.put_structure("pair", 2);
    let z = machine.set_variable();
    machine.set_value(z).unwrap();

    let right = machine.put_structure("pair", 2);
    machine.put_constant("one");
    machine.put_constant("two");

    assert!(!machine.unify(left, right).unwrap());
}

#[test]
fn compiles_ast_terms_to_m0_heap_and_unifies() {
    let query = PrologParser::parse_term("p(Z, h(Z, W), f(W))").unwrap();
    let fact = PrologParser::parse_term("p(a, h(a, b), f(b))").unwrap();
    let mut machine = M0Machine::new();
    let mut compiler = M0Compiler::new();

    let query_addr = compiler.compile_term(&mut machine, &query).unwrap();
    let fact_addr = M0Compiler::new().compile_term(&mut machine, &fact).unwrap();

    assert!(machine.unify(query_addr, fact_addr).unwrap());
}

#[test]
fn compiled_ast_terms_reject_inconsistent_bindings() {
    let query = PrologParser::parse_term("pair(X, X)").unwrap();
    let fact = PrologParser::parse_term("pair(one, two)").unwrap();
    let mut machine = M0Machine::new();
    let mut compiler = M0Compiler::new();

    let query_addr = compiler.compile_term(&mut machine, &query).unwrap();
    let fact_addr = M0Compiler::new().compile_term(&mut machine, &fact).unwrap();

    assert!(!machine.unify(query_addr, fact_addr).unwrap());
}
