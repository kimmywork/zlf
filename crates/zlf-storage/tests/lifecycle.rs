use std::collections::HashMap;
use std::sync::Arc;

use tempfile::TempDir;
use zlf_core::{Edge, EntityRef, Node, Value};
use zlf_storage::{MutationKind, Storage};

fn storage() -> (Storage, TempDir) {
    let temp = TempDir::new().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    (storage, temp)
}

fn node(id: &str) -> Node {
    Node::with_id(id.into(), vec!["person".into()], HashMap::new())
}

#[test]
fn mutations_publish_ordered_events_and_tombstones() {
    let (storage, _temp) = storage();
    storage.create_node(node("alice")).unwrap();
    storage
        .update_node(
            "alice",
            HashMap::from([("name".into(), Value::String("Alice".into()))]),
        )
        .unwrap();
    storage.delete_node("alice").unwrap();

    let events = storage.mutation_events_after(0, 10).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(
        events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(matches!(events[0].kind, MutationKind::Upsert { .. }));
    assert!(matches!(events[2].kind, MutationKind::Delete));

    let state = storage
        .get_entity_state(&EntityRef::Node("alice".into()))
        .unwrap()
        .unwrap();
    assert_eq!(state.source_version, 3);
    assert!(state.deleted);
}

#[test]
fn idempotent_label_write_does_not_publish_an_event() {
    let (storage, _temp) = storage();
    storage.create_node(node("alice")).unwrap();
    storage.add_labels("alice", &["person".into()]).unwrap();
    assert_eq!(storage.latest_mutation_sequence().unwrap(), 1);
}

#[test]
fn cascade_publishes_edge_tombstones_before_node_tombstone() {
    let (storage, _temp) = storage();
    storage.create_node(node("alice")).unwrap();
    storage.create_node(node("bob")).unwrap();
    let edge = storage
        .create_edge(Edge::with_id(
            "e1".into(),
            "knows".into(),
            "alice".into(),
            "bob".into(),
            HashMap::new(),
        ))
        .unwrap();

    storage.delete_node_cascade("alice").unwrap();
    let events = storage.mutation_events_after(3, 10).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].entity, Some(EntityRef::Edge(edge.id)));
    assert_eq!(events[1].entity, Some(EntityRef::Node("alice".into())));
    assert!(storage.get_edge("e1").unwrap().is_none());
    assert!(storage.get_node("alice").unwrap().is_none());
}

#[test]
fn reopen_preserves_outbox_and_watermark_source() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("db");
    {
        let storage = Storage::open(&path).unwrap();
        storage.create_node(node("alice")).unwrap();
    }
    let reopened = Storage::open_existing(&path).unwrap();
    assert_eq!(reopened.latest_mutation_sequence().unwrap(), 1);
    assert_eq!(reopened.mutation_events_after(0, 10).unwrap().len(), 1);
}

#[test]
fn concurrent_writers_allocate_unique_contiguous_sequences() {
    let temp = TempDir::new().unwrap();
    let storage = Arc::new(Storage::open(temp.path().join("db")).unwrap());
    let handles = (0..8)
        .map(|index| {
            let storage = Arc::clone(&storage);
            std::thread::spawn(move || storage.create_node(node(&format!("n{index}"))).unwrap())
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }
    let events = storage.mutation_events_after(0, 20).unwrap();
    assert_eq!(events.len(), 8);
    assert_eq!(
        events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        (1..=8).collect::<Vec<_>>()
    );
}
