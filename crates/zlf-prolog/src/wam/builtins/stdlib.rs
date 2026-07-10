use crate::parser::{PrologRule, Term};

pub(crate) fn core_library_rules() -> Vec<PrologRule> {
    let mut rules = member_rules();
    rules.extend(append_rules());
    rules.extend(select_rules());
    rules.extend(reverse_rules());
    rules.extend(control_rules());
    rules.extend(introspection_rules());
    rules
}

fn member_rules() -> Vec<PrologRule> {
    vec![
        rule(
            compound("member", vec![var("X"), cons(var("X"), var("Xs"))]),
            vec![],
        ),
        rule(
            compound("member", vec![var("X"), cons(var("_"), var("Xs"))]),
            vec![compound("member", vec![var("X"), var("Xs")])],
        ),
    ]
}

fn append_rules() -> Vec<PrologRule> {
    vec![
        rule(
            compound("append", vec![empty(), var("Ys"), var("Ys")]),
            vec![],
        ),
        rule(
            compound(
                "append",
                vec![
                    cons(var("X"), var("Xs")),
                    var("Ys"),
                    cons(var("X"), var("Zs")),
                ],
            ),
            vec![compound("append", vec![var("Xs"), var("Ys"), var("Zs")])],
        ),
    ]
}

fn select_rules() -> Vec<PrologRule> {
    vec![
        rule(
            compound(
                "select",
                vec![var("X"), cons(var("X"), var("Xs")), var("Xs")],
            ),
            vec![],
        ),
        rule(
            compound(
                "select",
                vec![
                    var("X"),
                    cons(var("Y"), var("Ys")),
                    cons(var("Y"), var("Zs")),
                ],
            ),
            vec![compound("select", vec![var("X"), var("Ys"), var("Zs")])],
        ),
    ]
}

fn reverse_rules() -> Vec<PrologRule> {
    vec![
        rule(
            compound("reverse", vec![var("Xs"), var("Ys")]),
            vec![compound(
                "$reverse_acc",
                vec![var("Xs"), empty(), var("Ys")],
            )],
        ),
        rule(
            compound("$reverse_acc", vec![empty(), var("Ys"), var("Ys")]),
            vec![],
        ),
        rule(
            compound(
                "$reverse_acc",
                vec![cons(var("X"), var("Xs")), var("Acc"), var("Ys")],
            ),
            vec![compound(
                "$reverse_acc",
                vec![var("Xs"), cons(var("X"), var("Acc")), var("Ys")],
            )],
        ),
    ]
}

fn control_rules() -> Vec<PrologRule> {
    vec![
        rule(
            compound("once", vec![var("Goal")]),
            vec![call(var("Goal")), atom("!")],
        ),
        rule(
            compound("\\+", vec![var("Goal")]),
            vec![call(var("Goal")), atom("!"), atom("fail")],
        ),
        rule(compound("\\+", vec![var("_")]), vec![]),
        rule(
            compound(";", vec![var("Left"), var("_")]),
            vec![call(var("Left"))],
        ),
        rule(
            compound(";", vec![var("_"), var("Right")]),
            vec![call(var("Right"))],
        ),
        rule(
            compound("->", vec![var("Condition"), var("Then")]),
            vec![call(var("Condition")), atom("!"), call(var("Then"))],
        ),
    ]
}

fn introspection_rules() -> Vec<PrologRule> {
    vec![
        rule(
            compound("current_predicate", vec![var("Indicator")]),
            vec![
                compound("nonvar", vec![var("Indicator")]),
                atom("!"),
                compound("$current_predicate_bound", vec![var("Indicator")]),
            ],
        ),
        rule(
            compound(
                "current_predicate",
                vec![compound("/", vec![var("Name"), var("Arity")])],
            ),
            vec![compound(
                "predicate",
                vec![var("Name"), var("Arity"), var("_")],
            )],
        ),
    ]
}

fn call(goal: Term) -> Term {
    compound("call", vec![goal])
}

fn rule(head: Term, body: Vec<Term>) -> PrologRule {
    PrologRule { head, body }
}

fn cons(head: Term, tail: Term) -> Term {
    compound(".", vec![head, tail])
}

fn atom(name: &str) -> Term {
    Term::Atom(name.to_string())
}

fn empty() -> Term {
    atom("[]")
}

fn var(name: &str) -> Term {
    Term::Variable(name.to_string())
}

fn compound(name: &str, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.to_string(),
        args,
    }
}
