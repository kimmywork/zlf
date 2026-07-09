use super::super::{Instruction, PredicateKey, WamExecutor, WamProgram};

#[test]
fn instruction_runtime_executes_parent_fact_call() {
    let mut executor = WamExecutor::new(2);
    let program = WamProgram::new(vec![
        Instruction::put_constant("alice", 0),
        Instruction::PutVariable { register: 1 },
        Instruction::Call(PredicateKey {
            name: "parent".to_string(),
            arity: 2,
        }),
        Instruction::get_constant("alice", 0),
        Instruction::get_constant("bob", 1),
        Instruction::Proceed,
    ]);

    let result = executor.execute(&program).unwrap();

    assert!(result.success);
    assert_eq!(executor.register_term(1).unwrap(), atom("bob"));
}

#[test]
fn instruction_runtime_rejects_constant_mismatch() {
    let mut executor = WamExecutor::new(1);
    let program = WamProgram::new(vec![
        Instruction::put_constant("alice", 0),
        Instruction::get_constant("bob", 0),
        Instruction::Proceed,
    ]);

    assert!(!executor.execute(&program).unwrap().success);
}

#[test]
fn instruction_runtime_builds_structure() {
    let mut executor = WamExecutor::new(1);
    let program = WamProgram::new(vec![
        Instruction::PutStructure {
            name: "pair".to_string(),
            arity: 2,
            register: 0,
        },
        Instruction::SetConstant {
            value: "one".to_string(),
        },
        Instruction::SetConstant {
            value: "two".to_string(),
        },
        Instruction::Proceed,
    ]);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(0).unwrap(), pair("one", "two"));
}

#[test]
fn instruction_runtime_reads_existing_structure_args() {
    let mut executor = WamExecutor::new(2);
    let program = WamProgram::new(vec![
        Instruction::PutVariable { register: 0 },
        Instruction::GetStructure {
            name: "pair".to_string(),
            arity: 2,
            register: 0,
        },
        Instruction::UnifyConstant {
            value: "one".to_string(),
        },
        Instruction::UnifyVariable { register: 1 },
        Instruction::get_constant("two", 1),
        Instruction::Proceed,
    ]);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(0).unwrap(), pair("one", "two"));
}

#[test]
fn instruction_runtime_call_jumps_to_predicate_entry() {
    let mut executor = WamExecutor::new(2);
    let key = PredicateKey {
        name: "parent".to_string(),
        arity: 2,
    };
    let program = WamProgram::new(vec![
        Instruction::put_constant("alice", 0),
        Instruction::PutVariable { register: 1 },
        Instruction::Call(key.clone()),
        Instruction::Proceed,
        Instruction::get_constant("carol", 0),
        Instruction::Proceed,
        Instruction::get_constant("alice", 0),
        Instruction::get_constant("bob", 1),
        Instruction::Proceed,
    ])
    .with_entry(key, 6);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(1).unwrap(), atom("bob"));
}

#[test]
fn instruction_runtime_allocate_and_deallocate_environment_frame() {
    let mut executor = WamExecutor::new(1);
    let program = WamProgram::new(vec![
        Instruction::Allocate,
        Instruction::Deallocate,
        Instruction::Proceed,
    ]);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.environment_depth(), 0);
}

#[test]
fn instruction_runtime_rejects_deallocate_without_frame() {
    let mut executor = WamExecutor::new(1);
    let program = WamProgram::new(vec![Instruction::Deallocate, Instruction::Proceed]);

    assert!(executor.execute(&program).is_err());
}

#[test]
fn instruction_runtime_leaves_allocated_frame_until_deallocate() {
    let mut executor = WamExecutor::new(1);
    let program = WamProgram::new(vec![Instruction::Allocate, Instruction::Proceed]);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.environment_depth(), 1);
}

#[test]
fn instruction_runtime_execute_all_collects_clause_solutions() {
    let mut executor = WamExecutor::new(1);
    let key = PredicateKey {
        name: "color".to_string(),
        arity: 1,
    };
    let program = WamProgram::new(vec![
        Instruction::PutVariable { register: 0 },
        Instruction::Call(key.clone()),
        Instruction::Proceed,
        Instruction::TryMeElse(6),
        Instruction::get_constant("red", 0),
        Instruction::Proceed,
        Instruction::RetryMeElse(9),
        Instruction::get_constant("green", 0),
        Instruction::Proceed,
        Instruction::TrustMe,
        Instruction::get_constant("blue", 0),
        Instruction::Proceed,
    ])
    .with_entry(key, 3);

    let solutions = executor.execute_all_registers(&program, &[0]).unwrap();

    assert_eq!(solutions.len(), 3);
    assert_eq!(solutions[0], vec![atom("red")]);
    assert_eq!(solutions[1], vec![atom("green")]);
    assert_eq!(solutions[2], vec![atom("blue")]);
}

#[test]
fn instruction_runtime_executes_rule_body_calls_with_call_stack() {
    let mut executor = WamExecutor::new(8);
    let parent = PredicateKey {
        name: "parent".to_string(),
        arity: 2,
    };
    let grandparent = PredicateKey {
        name: "grandparent".to_string(),
        arity: 2,
    };
    let program = grandparent_program(parent.clone(), grandparent.clone());

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(1).unwrap(), atom("carol"));
}

#[test]
fn instruction_runtime_backtracks_to_next_clause_on_failure() {
    let mut executor = WamExecutor::new(1);
    let key = PredicateKey {
        name: "color".to_string(),
        arity: 1,
    };
    let program = WamProgram::new(vec![
        Instruction::put_constant("green", 0),
        Instruction::Call(key.clone()),
        Instruction::Proceed,
        Instruction::TryMeElse(6),
        Instruction::get_constant("red", 0),
        Instruction::Proceed,
        Instruction::TrustMe,
        Instruction::get_constant("green", 0),
        Instruction::Proceed,
    ])
    .with_entry(key, 3);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(0).unwrap(), atom("green"));
    assert_eq!(executor.choice_point_depth(), 0);
}

#[test]
fn instruction_runtime_retry_restores_choice_point_state() {
    let mut executor = WamExecutor::new(1);
    let program = WamProgram::new(vec![
        Instruction::PutVariable { register: 0 },
        Instruction::TryMeElse(10),
        Instruction::get_constant("alice", 0),
        Instruction::RetryMeElse(20),
        Instruction::get_constant("bob", 0),
        Instruction::Proceed,
    ]);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(0).unwrap(), atom("bob"));
    assert_eq!(executor.choice_point_depth(), 1);
    assert_eq!(executor.next_alternative(), Some(20));
}

#[test]
fn instruction_runtime_trust_restores_and_discards_choice_point() {
    let mut executor = WamExecutor::new(1);
    let program = WamProgram::new(vec![
        Instruction::PutVariable { register: 0 },
        Instruction::TryMeElse(10),
        Instruction::get_constant("alice", 0),
        Instruction::TrustMe,
        Instruction::get_constant("bob", 0),
        Instruction::Proceed,
    ]);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(0).unwrap(), atom("bob"));
    assert_eq!(executor.choice_point_depth(), 0);
}

#[test]
fn instruction_runtime_rejects_retry_without_choice_point() {
    let mut executor = WamExecutor::new(1);
    let program = WamProgram::new(vec![Instruction::RetryMeElse(1), Instruction::Proceed]);

    assert!(executor.execute(&program).is_err());
}

#[test]
fn instruction_runtime_matches_existing_structure() {
    let mut executor = WamExecutor::new(2);
    let program = WamProgram::new(vec![
        Instruction::PutStructure {
            name: "pair".to_string(),
            arity: 2,
            register: 0,
        },
        Instruction::SetConstant {
            value: "one".to_string(),
        },
        Instruction::SetConstant {
            value: "two".to_string(),
        },
        Instruction::get_structure("pair", 2, 0),
        Instruction::unify_constant("one"),
        Instruction::UnifyVariable { register: 1 },
        Instruction::Proceed,
    ]);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(1).unwrap(), atom("two"));
}

#[allow(clippy::too_many_lines)]
fn grandparent_program(parent: PredicateKey, grandparent: PredicateKey) -> WamProgram {
    WamProgram::new(vec![
        Instruction::put_constant("alice", 0),
        Instruction::PutVariable { register: 1 },
        Instruction::Call(grandparent.clone()),
        Instruction::Proceed,
        Instruction::Allocate,
        Instruction::PutValue {
            source: 0,
            target: 2,
        },
        Instruction::PutValue {
            source: 1,
            target: 3,
        },
        Instruction::PutValue {
            source: 2,
            target: 0,
        },
        Instruction::PutVariable { register: 4 },
        Instruction::PutValue {
            source: 4,
            target: 1,
        },
        Instruction::Call(parent.clone()),
        Instruction::PutValue {
            source: 4,
            target: 0,
        },
        Instruction::PutValue {
            source: 3,
            target: 1,
        },
        Instruction::Call(parent.clone()),
        Instruction::Deallocate,
        Instruction::Proceed,
        Instruction::TryMeElse(20),
        Instruction::get_constant("alice", 0),
        Instruction::get_constant("bob", 1),
        Instruction::Proceed,
        Instruction::TrustMe,
        Instruction::get_constant("bob", 0),
        Instruction::get_constant("carol", 1),
        Instruction::Proceed,
    ])
    .with_entry(grandparent, 4)
    .with_entry(parent, 16)
}

fn atom(value: &str) -> crate::Term {
    crate::Term::Atom(value.to_string())
}

fn pair(left: &str, right: &str) -> crate::Term {
    crate::Term::Compound {
        name: "pair".to_string(),
        args: vec![atom(left), atom(right)],
    }
}
