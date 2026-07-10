use std::sync::Arc;

use tempfile::tempdir;
use zlf_prolog::wam::{
    FactKey, PredicateKey, RocksTableBackend, StorageFactProvider, StorageFactWriter, TableLimits,
    TableManager, WamRuntime,
};
use zlf_prolog::{PrologParser, Term};
use zlf_storage::Storage;

#[test]
fn complete_tables_survive_a_fresh_hot_manager() {
    let temp = tempdir().unwrap();
    let storage = Arc::new(Storage::open(temp.path().join("db")).unwrap());
    let first_manager = manager(&storage);
    let first = runtime(Arc::clone(&first_manager));
    let query = term("reachable(a, X)");
    assert_eq!(first.query_all(&query).unwrap().len(), 3);
    assert_eq!(first_manager.metrics().tables_completed, 1);

    let restarted_manager = manager(&storage);
    let restarted = runtime(Arc::clone(&restarted_manager));
    assert_eq!(restarted.query_all(&query).unwrap().len(), 3);
    assert_eq!(restarted_manager.metrics().persistent_hits, 1);
    assert_eq!(restarted_manager.metrics().tables_completed, 0);
}

#[test]
fn complete_hot_tables_evict_to_and_reload_from_rocksdb() {
    let temp = tempdir().unwrap();
    let storage = Arc::new(Storage::open(temp.path().join("db")).unwrap());
    let manager = Arc::new(TableManager::with_backend_and_limits(
        Arc::new(RocksTableBackend::new(Arc::clone(&storage))),
        TableLimits {
            max_tables: 2,
            ..TableLimits::default()
        },
    ));
    let runtime = runtime(Arc::clone(&manager));
    for source in ["a", "b", "c"] {
        runtime
            .query_all(&term(&format!("reachable({source}, X)")))
            .unwrap();
    }
    runtime.query_all(&term("reachable(a, X)")).unwrap();
    assert!(manager.metrics().evictions > 0);
    assert!(manager.metrics().persistent_hits > 0);
}

#[test]
fn exact_fact_dependencies_preserve_other_variants_of_the_same_predicate() {
    let temp = tempdir().unwrap();
    let storage = Arc::new(Storage::open(temp.path().join("db")).unwrap());
    write_edges(&storage, &["edge(a,b)", "edge(b,c)", "edge(x,y)"]);
    let provider = StorageFactProvider::new(&storage);
    let first_manager = manager(&storage);
    let first = rules_runtime(Arc::clone(&first_manager));
    first
        .query_all_with_provider_and_storage(&term("reachable(a,X)"), &provider, &storage)
        .unwrap();
    first
        .query_all_with_provider_and_storage(&term("reachable(x,X)"), &provider, &storage)
        .unwrap();
    first_manager
        .invalidate_facts(&[FactKey::Edge {
            source: "a".to_string(),
            edge_type: "edge".to_string(),
            target: "b".to_string(),
        }])
        .unwrap();

    let restarted_manager = manager(&storage);
    let restarted = rules_runtime(Arc::clone(&restarted_manager));
    restarted
        .query_all_with_provider_and_storage(&term("reachable(x,X)"), &provider, &storage)
        .unwrap();
    assert_eq!(restarted_manager.metrics().persistent_hits, 1);
    restarted
        .query_all_with_provider_and_storage(&term("reachable(a,X)"), &provider, &storage)
        .unwrap();
    assert_eq!(restarted_manager.metrics().tables_completed, 1);
}

#[test]
fn persistent_tables_become_stale_and_recompute_after_invalidation() {
    let temp = tempdir().unwrap();
    let storage = Arc::new(Storage::open(temp.path().join("db")).unwrap());
    let first_manager = manager(&storage);
    assert_eq!(
        runtime(Arc::clone(&first_manager))
            .query_all(&term("reachable(a, X)"))
            .unwrap()
            .len(),
        3
    );
    first_manager.invalidate_all().unwrap();

    let restarted_manager = manager(&storage);
    assert_eq!(
        runtime(Arc::clone(&restarted_manager))
            .query_all(&term("reachable(a, X)"))
            .unwrap()
            .len(),
        3
    );
    assert_eq!(restarted_manager.metrics().persistent_hits, 0);
    assert_eq!(restarted_manager.metrics().tables_completed, 1);
}

fn write_edges(storage: &Storage, edges: &[&str]) {
    let writer = StorageFactWriter::new(storage);
    for edge in edges {
        writer.apply_fact(&term(edge)).unwrap();
    }
}

fn manager(storage: &Arc<Storage>) -> Arc<TableManager> {
    Arc::new(TableManager::with_backend(Arc::new(
        RocksTableBackend::new(Arc::clone(storage)),
    )))
}

fn runtime(manager: Arc<TableManager>) -> WamRuntime {
    let mut runtime = rules_runtime(manager);
    for edge in ["edge(a,b)", "edge(b,c)", "edge(c,d)"] {
        runtime.add_fact(term(edge));
    }
    runtime
}

fn rules_runtime(manager: Arc<TableManager>) -> WamRuntime {
    let mut runtime = WamRuntime::new(64);
    runtime.set_table_manager(manager);
    runtime.add_rule(PrologParser::parse_rule("reachable(X,Y) :- edge(X,Y).").unwrap());
    runtime.add_rule(
        PrologParser::parse_rule("reachable(X,Y) :- reachable(X,Z), edge(Z,Y).").unwrap(),
    );
    runtime.declare_tabled(PredicateKey {
        name: "reachable".to_string(),
        arity: 2,
    });
    runtime
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}
