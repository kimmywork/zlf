use super::super::{Instruction, WamExecutor, WamProgram};

#[test]
fn instruction_runtime_switch_on_constant_jumps_to_matching_case() {
    let mut executor = WamExecutor::new(1);
    let program = WamProgram::new(vec![
        Instruction::put_constant("green", 0),
        Instruction::SwitchOnConstant {
            register: 0,
            cases: vec![("red".to_string(), 2), ("green".to_string(), 4)],
            default: Some(6),
        },
        Instruction::get_constant("red", 0),
        Instruction::Proceed,
        Instruction::get_constant("green", 0),
        Instruction::Proceed,
        Instruction::get_constant("blue", 0),
        Instruction::Proceed,
    ]);

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(0).unwrap(), atom("green"));
}

#[test]
fn instruction_runtime_switch_on_term_and_structure_jump_by_type() {
    let mut executor = WamExecutor::new(1);
    let program = pair_switch_program();

    assert!(executor.execute(&program).unwrap().success);
    assert_eq!(executor.register_term(0).unwrap(), pair("a", "b"));
}

fn pair_switch_program() -> WamProgram {
    WamProgram::new(vec![
        put_pair(),
        Instruction::SetConstant { value: "a".into() },
        Instruction::SetConstant { value: "b".into() },
        switch_on_structure_type(),
        switch_on_pair(),
        get_pair(),
        Instruction::unify_constant("a"),
        Instruction::unify_constant("b"),
        Instruction::Proceed,
    ])
}

fn put_pair() -> Instruction {
    Instruction::PutStructure {
        name: "pair".into(),
        arity: 2,
        register: 0,
    }
}

fn switch_on_structure_type() -> Instruction {
    Instruction::SwitchOnTerm {
        register: 0,
        variable: None,
        constant: None,
        list: None,
        structure: Some(4),
    }
}

fn switch_on_pair() -> Instruction {
    Instruction::SwitchOnStructure {
        register: 0,
        cases: vec![("pair".into(), 2, 5)],
        default: None,
    }
}

fn get_pair() -> Instruction {
    Instruction::GetStructure {
        name: "pair".into(),
        arity: 2,
        register: 0,
    }
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
