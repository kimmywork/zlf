use zlf_prolog::wam::{StorageFactProvider, StorageFactWriter, WamRuntime};
use zlf_prolog::{PrologParser, Term};
use zlf_storage::Storage;

#[test]
fn storage_writer_adds_label_shortcut_to_existing_node() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep().join("db");
    let storage = Storage::open(&path).unwrap();
    let writer = StorageFactWriter::new(&storage);

    writer.apply_fact(&term("node(bob)")).unwrap();
    writer.apply_fact(&term("person(bob)")).unwrap();

    let bob = storage.get_node("bob").unwrap().unwrap();
    assert!(bob.has_label("person"));

    let provider = StorageFactProvider::new(&storage);
    let runtime = WamRuntime::new(12);
    let solutions = runtime
        .query_all_with_provider(&term("person(X)"), &provider)
        .unwrap();
    assert_eq!(solutions[0].get("X"), Some(&atom("bob")));
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}
