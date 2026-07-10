use zlf_prolog::{PrologParser, Query, Term};

#[test]
fn parse_basic_terms() {
    assert!(matches!(
        PrologParser::parse_term("X").unwrap(),
        Term::Variable(_)
    ));
    assert!(matches!(
        PrologParser::parse_term("alice").unwrap(),
        Term::Atom(_)
    ));
    assert!(matches!(
        PrologParser::parse_term("42").unwrap(),
        Term::Integer(_)
    ));
    assert!(matches!(
        PrologParser::parse_term("42.5").unwrap(),
        Term::Float(_)
    ));
    assert!(matches!(
        PrologParser::parse_term("\"hello\"").unwrap(),
        Term::String(_)
    ));
}

#[test]
fn parse_compound_list_and_object() {
    let compound = PrologParser::parse_term("parent(alice, bob)").unwrap();
    assert!(matches!(compound, Term::Compound { .. }));

    let list = PrologParser::parse_term("[person, developer]").unwrap();
    assert!(matches!(list, Term::List(items) if items.len() == 2));

    let object = PrologParser::parse_term("{ name: \"Alice\", age: 17 }").unwrap();
    assert!(matches!(object, Term::Object(entries) if entries.len() == 2));

    assert_eq!(
        PrologParser::parse_term("!").unwrap(),
        Term::Atom("!".into())
    );
}

#[test]
fn parse_fact_rule_and_queries() {
    let fact = PrologParser::parse_fact("parent(alice, bob).").unwrap();
    assert_eq!(fact.head.predicate_name(), "parent");

    let rule =
        PrologParser::parse_rule("grandparent(X, Z) :- parent(X, Y), parent(Y, Z).").unwrap();
    assert_eq!(rule.body.len(), 2);

    let cut_rule = PrologParser::parse_rule("first_color(X) :- color(X), !.").unwrap();
    assert_eq!(cut_rule.body[1], Term::Atom("!".into()));

    assert!(matches!(
        PrologParser::parse_query("?parent(alice, X).").unwrap(),
        Query::Goal(_)
    ));
    assert!(matches!(
        PrologParser::parse_query("?parent(alice, X), parent(X, Y).").unwrap(),
        Query::Goals(_)
    ));
}

#[test]
fn parse_invalid_syntax_fails() {
    assert!(PrologParser::parse_query("invalid query syntax").is_err());
}
