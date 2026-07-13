use zlf_prolog::PrologParser;
use zlf_query::{AccessPath, ArgumentMode, ZlfDatabase};

#[test]
fn explain_distinguishes_event_and_validity_index_paths() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
    let event = db
        .explain_prolog("? temporal_between(\"2026-01-01\", \"2026-02-01\", Node).")
        .unwrap();
    let validity = db
        .explain_prolog("? valid_at(\"2026-01-15T00:00:00Z\", Node).")
        .unwrap();
    assert_eq!(event.goals[0].access, AccessPath::TemporalEventRange);
    assert_eq!(validity.goals[0].access, AccessPath::ValidityInterval);
    assert!(event.goals[0].pushed_down);
    assert!(validity.goals[0].pushed_down);
}

#[test]
fn explain_identifies_prepared_hybrid_retrieval() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
    let plan = db
        .explain_prolog("? retrieve(\"prepared-retrieval-1\", {}, Entity, Hit).")
        .unwrap();
    assert_eq!(plan.goals[0].access, AccessPath::HybridRetrieval);
    assert!(plan.goals[0].pushed_down);
}

#[test]
fn explain_exposes_bound_storage_pushdown_across_goals() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    for fact in [
        "node(a, [taxon], {rank: species}).",
        "node(b, [taxon], {rank: genus}).",
        "edge(a, next, b).",
    ] {
        db.apply_fact(&PrologParser::parse_fact(fact).unwrap().head)
            .unwrap();
    }
    let plan = db
        .explain_prolog("? next(a,X), prop_rank(X,Rank).")
        .unwrap();
    assert_eq!(
        plan.goals[0].modes,
        vec![ArgumentMode::Bound, ArgumentMode::Free]
    );
    assert_eq!(plan.goals[0].access, AccessPath::OutgoingEdges);
    assert!(plan.goals[0].pushed_down);
    assert_eq!(
        plan.goals[1].modes,
        vec![ArgumentMode::Bound, ArgumentMode::Free]
    );
    assert_eq!(plan.goals[1].access, AccessPath::EntityProperty);
}

#[test]
fn explain_distinguishes_exact_property_and_unbound_edge_scans() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();
    db.apply_fact(
        &PrologParser::parse_fact("node(a, [taxon], {rank: species}).")
            .unwrap()
            .head,
    )
    .unwrap();
    db.apply_fact(&PrologParser::parse_fact("edge(a, next, b).").unwrap().head)
        .unwrap();

    let exact = db
        .explain_prolog("? prop_rank(Taxon, \"species\").")
        .unwrap();
    assert_eq!(exact.goals[0].access, AccessPath::ExactProperty);
    let scan = db.explain_prolog("? next(Source,Target).").unwrap();
    assert_eq!(scan.goals[0].access, AccessPath::EdgeTypeScan);
}
