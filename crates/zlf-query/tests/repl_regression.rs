use zlf_prolog::PrologParser;
use zlf_query::ZlfDatabase;

#[test]
fn multi_goal_query_preserves_bindings_with_rule_and_property_shortcuts() {
    let dir = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(dir.path()).unwrap();

    db.apply_fact(
        &PrologParser::parse_fact("node(zlf, [person, wanghong], { name: \"峰哥亡命天涯\" }).")
            .unwrap()
            .head,
    )
    .unwrap();
    db.apply_fact(
        &PrologParser::parse_fact("node(tongtong, [person, wanghong], { name: \"散仙彤彤子\" }).")
            .unwrap()
            .head,
    )
    .unwrap();
    db.apply_fact(
        &PrologParser::parse_fact("knows(zlf, tongtong).")
            .unwrap()
            .head,
    )
    .unwrap();
    db.query_prolog("friend(X, Y) :- person(X), person(Y), knows(X, Y).")
        .unwrap();

    let rows = db
        .query_prolog("?friend(zlf, X), prop_name(zlf, Y), prop_name(X, Z).")
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["X"], "tongtong");
    assert_eq!(rows[0]["Y"], "峰哥亡命天涯");
    assert_eq!(rows[0]["Z"], "散仙彤彤子");
}
