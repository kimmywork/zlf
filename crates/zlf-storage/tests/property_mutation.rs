use std::collections::{BTreeMap, BTreeSet, HashMap};

use tempfile::TempDir;
use zlf_core::{Edge, Node, PropertyPatch, Value, ZlfError};
use zlf_storage::Storage;

fn fixture() -> (Storage, TempDir) {
    let temp = TempDir::new().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage
        .create_node(Node::with_id(
            "alice".into(),
            vec!["person".into()],
            HashMap::from([
                ("name".into(), Value::String("Alice".into())),
                ("keep".into(), Value::Bool(true)),
            ]),
        ))
        .unwrap();
    storage
        .create_node(Node::with_id("bob".into(), vec![], HashMap::new()))
        .unwrap();
    (storage, temp)
}

#[test]
fn node_patch_is_atomic_preserves_other_values_and_keeps_null() {
    let (storage, _temp) = fixture();
    let patch = PropertyPatch {
        set: BTreeMap::from([
            ("name".into(), Value::String("Alicia".into())),
            ("nullable".into(), Value::Null),
        ]),
        remove: BTreeSet::from(["missing".into()]),
    };
    let receipt = storage.patch_node_properties("alice", &patch).unwrap();
    assert_eq!(receipt.sequence, Some(3));
    let node = storage.get_node("alice").unwrap().unwrap();
    assert_eq!(node.properties["name"], Value::String("Alicia".into()));
    assert_eq!(node.properties["keep"], Value::Bool(true));
    assert_eq!(node.properties["nullable"], Value::Null);
}

#[test]
fn missing_remove_and_identical_set_are_event_free() {
    let (storage, _temp) = fixture();
    assert!(storage
        .remove_node_property("alice", "missing")
        .unwrap()
        .sequence
        .is_none());
    assert!(storage
        .set_node_property("alice", "name", Value::String("Alice".into()))
        .unwrap()
        .sequence
        .is_none());
    assert_eq!(storage.latest_mutation_sequence().unwrap(), 2);
}

#[test]
fn edge_properties_mutate_without_changing_relation_identity() {
    let (storage, _temp) = fixture();
    storage
        .create_edge(Edge::with_id(
            "e1".into(),
            "knows".into(),
            "alice".into(),
            "bob".into(),
            HashMap::from([("confidence".into(), Value::Number(0.5))]),
        ))
        .unwrap();
    storage
        .set_edge_property("e1", "confidence", Value::Number(0.9))
        .unwrap();
    let edge = storage.get_edge("e1").unwrap().unwrap();
    assert_eq!(edge.properties["confidence"], Value::Number(0.9));
    assert_eq!(
        (
            edge.source.as_str(),
            edge.edge_type.as_str(),
            edge.target.as_str()
        ),
        ("alice", "knows", "bob")
    );
}

#[test]
fn generic_property_mutation_rejects_ambiguous_ids() {
    let (storage, _temp) = fixture();
    storage
        .create_edge(Edge::with_id(
            "alice".into(),
            "knows".into(),
            "alice".into(),
            "bob".into(),
            HashMap::new(),
        ))
        .unwrap();
    assert!(matches!(
        storage.set_entity_property("alice", "x", Value::Bool(true)),
        Err(ZlfError::AmbiguousEntity(id)) if id == "alice"
    ));
}

#[test]
fn parallel_edge_ids_are_returned_in_stable_order() {
    let (storage, _temp) = fixture();
    for id in ["e2", "e1"] {
        storage
            .create_edge(Edge::with_id(
                id.into(),
                "knows".into(),
                "alice".into(),
                "bob".into(),
                HashMap::new(),
            ))
            .unwrap();
    }
    assert_eq!(
        storage.get_edge_ids("alice", "knows", "bob").unwrap(),
        vec!["e1".to_string(), "e2".to_string()]
    );
}
