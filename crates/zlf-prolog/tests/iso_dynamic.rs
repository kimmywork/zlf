use zlf_prolog::wam::{
    CompiledRuleArtifact, CompositeFactProvider, IntrospectionProvider, PredicateKey,
    PredicateKind, PredicateRegistry, StorageFactProvider, StorageRuleStore, WamRuntime,
};
use zlf_prolog::{PrologRule, Term};
use zlf_storage::Storage;

#[test]
#[allow(clippy::too_many_lines)]
fn wam_dynamic_builtins_assert_retract_and_enumerate_predicates() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("storage")).unwrap();
    let runtime = WamRuntime::new(64);
    let storage_provider = StorageFactProvider::new(&storage);
    let provider = CompositeFactProvider::new().with(&storage_provider);

    for name in ["asserta", "assertz"] {
        assert_eq!(
            run(
                &runtime,
                &provider,
                &storage,
                compound(
                    name,
                    vec![compound("likes", vec![atom("alice"), atom(name)])]
                ),
            )
            .len(),
            1
        );
    }
    assert_eq!(
        run(
            &runtime,
            &provider,
            &storage,
            compound("current_predicate", vec![indicator("likes", 2)]),
        )
        .len(),
        1
    );
    assert_eq!(
        run(
            &runtime,
            &provider,
            &storage,
            compound(
                "retract",
                vec![compound("likes", vec![atom("alice"), atom("asserta")])],
            ),
        )
        .len(),
        1
    );
    assert_eq!(
        run(
            &runtime,
            &provider,
            &storage,
            compound(
                "retractall",
                vec![compound("likes", vec![atom("alice"), var("_")])],
            ),
        )
        .len(),
        1
    );
    assert!(run(
        &runtime,
        &provider,
        &storage,
        compound("likes", vec![atom("alice"), var("X")]),
    )
    .is_empty());
}

#[test]
fn current_predicate_enumerates_through_registry_rules() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("storage")).unwrap();
    let mut registry = PredicateRegistry::new();
    registry.register(key("custom", 2), PredicateKind::UserRule);
    let rules = Vec::<CompiledRuleArtifact>::new();
    let introspection = IntrospectionProvider::new(registry, &rules);
    let provider = CompositeFactProvider::new().with(&introspection);
    let rows = WamRuntime::new(64)
        .query_all_with_provider_and_storage(
            &compound("current_predicate", vec![var("Indicator")]),
            &provider,
            &storage,
        )
        .unwrap();
    assert!(rows
        .iter()
        .any(|row| row["Indicator"] == indicator("custom", 2)));
}

#[test]
#[allow(clippy::too_many_lines)]
fn assert_rule_clause_and_retract_rule_use_storage_rule_store() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("storage")).unwrap();
    let runtime = WamRuntime::new(64);
    let provider = CompositeFactProvider::new();
    let rule = PrologRule {
        head: compound("friend", vec![var("X"), var("Y")]),
        body: vec![compound("knows", vec![var("X"), var("Y")])],
    };
    let rule_term = compound(":-", vec![rule.head.clone(), rule.body[0].clone()]);

    assert_eq!(
        run(
            &runtime,
            &provider,
            &storage,
            compound("assertz", vec![rule_term.clone()]),
        )
        .len(),
        1
    );
    assert_eq!(
        StorageRuleStore::new(&storage).all_rules().unwrap().len(),
        1
    );

    let clause = run(
        &runtime,
        &provider,
        &storage,
        compound("clause", vec![rule.head.clone(), var("Body")]),
    );
    assert!(matches!(
        &clause[0]["Body"],
        Term::Compound { name, args } if name == "knows" && args.len() == 2
    ));

    assert_eq!(
        run(
            &runtime,
            &provider,
            &storage,
            compound("retract", vec![rule_term]),
        )
        .len(),
        1
    );
    assert!(StorageRuleStore::new(&storage)
        .all_rules()
        .unwrap()
        .is_empty());
}

fn run(
    runtime: &WamRuntime,
    provider: &CompositeFactProvider<'_>,
    storage: &Storage,
    term: Term,
) -> Vec<std::collections::HashMap<String, Term>> {
    runtime
        .query_all_with_provider_and_storage(&term, provider, storage)
        .unwrap()
}

fn indicator(name: &str, arity: i64) -> Term {
    compound("/", vec![atom(name), Term::Integer(arity)])
}

fn key(name: &str, arity: usize) -> PredicateKey {
    PredicateKey {
        name: name.to_string(),
        arity,
    }
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}

fn var(value: &str) -> Term {
    Term::Variable(value.to_string())
}

fn compound(name: &str, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.to_string(),
        args,
    }
}
