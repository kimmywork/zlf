use tempfile::tempdir;
use zlf_prolog::wam::{FactProvider, StorageFactProvider, StorageFactWriter};
use zlf_prolog::{PrologParser, Term};
use zlf_storage::Storage;

#[test]
fn bound_edge_goals_use_directional_storage_lookups() {
    let temp = tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    let writer = StorageFactWriter::new(&storage);
    for fact in [
        "node(a)",
        "node(b)",
        "node(c)",
        "taxonomy_parent(a,b)",
        "taxonomy_parent(c,b)",
    ] {
        writer.apply_fact(&term(fact)).unwrap();
    }
    let provider = StorageFactProvider::new(&storage);
    assert_eq!(
        provider
            .facts_for_goal(&term("taxonomy_parent(a, Parent)"))
            .unwrap(),
        vec![term("taxonomy_parent(a,b)")]
    );
    let incoming = provider
        .facts_for_goal(&term("taxonomy_parent(Child, b)"))
        .unwrap();
    assert_eq!(incoming.len(), 2);
    assert!(incoming.contains(&term("taxonomy_parent(a,b)")));
    assert!(incoming.contains(&term("taxonomy_parent(c,b)")));
}

#[test]
fn bound_label_and_property_shortcuts_avoid_relation_materialization() {
    let temp = tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    let writer = StorageFactWriter::new(&storage);
    writer
        .apply_fact(&term("node(tax_1, [taxon], {rank: species})"))
        .unwrap();
    let provider = StorageFactProvider::new(&storage);
    assert_eq!(
        provider.facts_for_goal(&term("taxon(tax_1)")).unwrap(),
        vec![term("taxon(tax_1)")]
    );
    assert_eq!(
        provider
            .facts_for_goal(&term("prop_rank(Taxon, \"species\")"))
            .unwrap(),
        vec![Term::Compound {
            name: "prop_rank".to_string(),
            args: vec![atom("tax_1"), Term::String("species".to_string())],
        }]
    );
    assert_eq!(
        provider
            .facts_for_goal(&term("prop_rank(tax_1, Rank)"))
            .unwrap(),
        vec![Term::Compound {
            name: "prop_rank".to_string(),
            args: vec![atom("tax_1"), Term::String("species".to_string())],
        }]
    );
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}
