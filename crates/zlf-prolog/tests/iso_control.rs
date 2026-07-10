use zlf_prolog::wam::WamRuntime;
use zlf_prolog::{PrologParser, Term};

#[test]
fn wam_meta_call_supports_call_1_through_call_8() {
    let mut runtime = WamRuntime::new(64);
    for extra_count in 0..=7 {
        let name = format!("call_target_{extra_count}");
        let args = (0..extra_count)
            .map(|index| atom(&format!("a{index}")))
            .collect::<Vec<_>>();
        let fact = if args.is_empty() {
            atom(&name)
        } else {
            compound(&name, args.clone())
        };
        runtime.add_fact(fact);
        let mut call_args = vec![atom(&name)];
        call_args.extend(args);
        let rows = runtime.query_all(&compound("call", call_args)).unwrap();
        assert_eq!(rows.len(), 1, "call/{}", extra_count + 1);
    }
}

#[test]
fn wam_control_rules_preserve_choices_and_cut_scope() {
    let runtime = WamRuntime::new(64);
    let once = query(&runtime, "once(member(X, [a,b,c]))");
    assert_eq!(once.len(), 1);
    assert_eq!(once[0]["X"], atom("a"));

    assert_eq!(query(&runtime, "\\+ member(z, [a,b,c])").len(), 1);
    assert!(query(&runtime, "\\+ member(a, [a,b,c])").is_empty());

    let either = query(&runtime, "member(X, [a]); member(X, [b])");
    assert_eq!(either.len(), 2);
    assert_eq!(either[0]["X"], atom("a"));
    assert_eq!(either[1]["X"], atom("b"));

    assert_eq!(query(&runtime, "member(a, [a]) -> true").len(), 1);
    assert!(query(&runtime, "member(z, [a]) -> true").is_empty());
}

#[test]
fn true_fail_and_false_execute_in_the_wam() {
    let runtime = WamRuntime::new(16);
    assert_eq!(runtime.query_all(&atom("true")).unwrap().len(), 1);
    assert!(runtime.query_all(&atom("fail")).unwrap().is_empty());
    assert!(runtime.query_all(&atom("false")).unwrap().is_empty());
}

fn query(runtime: &WamRuntime, source: &str) -> Vec<std::collections::HashMap<String, Term>> {
    runtime
        .query_all(&PrologParser::parse_term(source).unwrap())
        .unwrap()
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}

fn compound(name: &str, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.to_string(),
        args,
    }
}
