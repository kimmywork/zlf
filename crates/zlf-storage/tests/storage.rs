use std::collections::HashMap;

use chrono::Utc;
use tempfile::TempDir;
use zlf_core::{Edge, Node, Value, ZlfError};
use zlf_storage::*;

fn create_test_storage() -> (Storage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::open(temp_dir.path().join("test.db")).unwrap();
    (storage, temp_dir)
}

fn create_test_node(id: &str) -> Node {
    let mut props = HashMap::new();
    props.insert("name".to_string(), Value::String("Alice".to_string()));
    Node::with_id(id.to_string(), vec!["person".to_string()], props)
}

#[test]
fn test_create_and_get_node() {
    let (storage, _temp) = create_test_storage();
    let node = create_test_node("alice");

    let created = storage.create_node(node.clone()).unwrap();
    assert_eq!(created.id, "alice");

    let retrieved = storage.get_node("alice").unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, "alice");
}

#[test]
fn test_duplicate_node_id() {
    let (storage, _temp) = create_test_storage();
    let node = create_test_node("alice");

    storage.create_node(node.clone()).unwrap();

    let result = storage.create_node(node);
    assert!(matches!(result, Err(ZlfError::NodeAlreadyExists(_))));
}

#[test]
fn test_node_not_found() {
    let (storage, _temp) = create_test_storage();

    let result = storage.get_node("nonexistent");
    assert!(result.unwrap().is_none());
}

#[test]
fn test_update_node() {
    let (storage, _temp) = create_test_storage();
    let node = create_test_node("alice");

    storage.create_node(node).unwrap();

    let mut new_props = HashMap::new();
    new_props.insert(
        "name".to_string(),
        Value::String("Alice Updated".to_string()),
    );

    let updated = storage.update_node("alice", new_props).unwrap();
    assert_eq!(updated.current_version, 2);
    assert_eq!(
        updated.properties.get("name"),
        Some(&Value::String("Alice Updated".to_string()))
    );
}

#[test]
fn test_delete_node() {
    let (storage, _temp) = create_test_storage();
    let node = create_test_node("alice");

    storage.create_node(node).unwrap();

    let deleted = storage.delete_node("alice").unwrap();
    assert!(deleted);

    let retrieved = storage.get_node("alice").unwrap();
    assert!(retrieved.is_none());
}

#[test]
fn test_create_edge() {
    let (storage, _temp) = create_test_storage();

    let node1 = create_test_node("alice");
    let node2 = create_test_node("bob");

    storage.create_node(node1).unwrap();
    storage.create_node(node2).unwrap();

    let mut props = HashMap::new();
    props.insert("since".to_string(), Value::Number(2020.0));

    let edge = Edge::new(
        "knows".to_string(),
        "alice".to_string(),
        "bob".to_string(),
        props,
    );

    let created = storage.create_edge(edge).unwrap();
    assert_eq!(created.edge_type, "knows");
    assert_eq!(created.source, "alice");
    assert_eq!(created.target, "bob");
}

#[test]
fn test_uuid_edge_id_adjacency_indexes_round_trip() {
    let (storage, _temp) = create_test_storage();
    storage.create_node(create_test_node("alice")).unwrap();
    storage.create_node(create_test_node("bob")).unwrap();

    let created = storage
        .create_edge(Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        ))
        .unwrap();

    let outgoing = storage.get_outgoing_edges("alice", Some("knows")).unwrap();
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].id, created.id);

    assert!(storage
        .delete_edge_by_triple("alice", "knows", "bob")
        .unwrap());
    assert!(storage
        .get_outgoing_edges("alice", Some("knows"))
        .unwrap()
        .is_empty());
}

#[test]
fn test_empty_edge_type() {
    let (storage, _temp) = create_test_storage();

    let node1 = create_test_node("alice");
    let node2 = create_test_node("bob");

    storage.create_node(node1).unwrap();
    storage.create_node(node2).unwrap();

    let edge = Edge::new(
        String::new(),
        "alice".to_string(),
        "bob".to_string(),
        HashMap::new(),
    );

    let result = storage.create_edge(edge);
    assert!(matches!(result, Err(ZlfError::EmptyEdgeType)));
}

#[test]
fn test_source_node_not_found() {
    let (storage, _temp) = create_test_storage();

    let node2 = create_test_node("bob");
    storage.create_node(node2).unwrap();

    let edge = Edge::new(
        "knows".to_string(),
        "alice".to_string(),
        "bob".to_string(),
        HashMap::new(),
    );

    let result = storage.create_edge(edge);
    assert!(matches!(result, Err(ZlfError::SourceNodeNotFound(_))));
}

#[test]
fn test_target_node_not_found() {
    let (storage, _temp) = create_test_storage();

    let node1 = create_test_node("alice");
    storage.create_node(node1).unwrap();

    let edge = Edge::new(
        "knows".to_string(),
        "alice".to_string(),
        "bob".to_string(),
        HashMap::new(),
    );

    let result = storage.create_edge(edge);
    assert!(matches!(result, Err(ZlfError::TargetNodeNotFound(_))));
}

#[test]
fn test_get_nodes_by_label() {
    let (storage, _temp) = create_test_storage();

    let node1 = create_test_node("alice");
    let node2 = create_test_node("bob");
    let node3 = Node::with_id(
        "acme".to_string(),
        vec!["company".to_string()],
        HashMap::new(),
    );

    storage.create_node(node1).unwrap();
    storage.create_node(node2).unwrap();
    storage.create_node(node3).unwrap();

    let persons = storage.get_nodes_by_label("person").unwrap();
    assert_eq!(persons.len(), 2);

    let companies = storage.get_nodes_by_label("company").unwrap();
    assert_eq!(companies.len(), 1);
}

#[test]
fn test_get_all_edges() {
    let (storage, _temp) = create_test_storage();
    storage.create_node(create_test_node("alice")).unwrap();
    storage.create_node(create_test_node("bob")).unwrap();
    storage.create_node(create_test_node("charlie")).unwrap();

    storage
        .create_edge(Edge::new(
            "knows".to_string(),
            "alice".to_string(),
            "bob".to_string(),
            HashMap::new(),
        ))
        .unwrap();
    storage
        .create_edge(Edge::new(
            "knows".to_string(),
            "bob".to_string(),
            "charlie".to_string(),
            HashMap::new(),
        ))
        .unwrap();

    let edges = storage.get_all_edges().unwrap();
    assert_eq!(edges.len(), 2);
}

#[test]
fn test_update_with_same_properties_creates_version() {
    let (storage, _temp) = create_test_storage();
    let node = create_test_node("alice");

    storage.create_node(node).unwrap();

    let mut props = HashMap::new();
    props.insert("name".to_string(), Value::String("Alice".to_string()));

    let updated = storage.update_node("alice", props).unwrap();
    assert_eq!(updated.current_version, 2);
}

#[test]
fn test_get_node_versions() {
    let (storage, _temp) = create_test_storage();
    let node = create_test_node("alice");

    storage.create_node(node).unwrap();

    let mut props = HashMap::new();
    props.insert(
        "name".to_string(),
        Value::String("Alice Updated".to_string()),
    );

    storage.update_node("alice", props).unwrap();

    let versions = storage.get_node_versions("alice").unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version_id, 1);
    assert_eq!(versions[1].version_id, 2);
}

#[test]
fn test_get_node_at_time() {
    let (storage, _temp) = create_test_storage();
    let node = create_test_node("alice");

    storage.create_node(node).unwrap();

    let now = Utc::now();
    let node_at_time = storage.get_node_at_time("alice", now).unwrap();
    assert!(node_at_time.is_some());
}

#[test]
fn compiled_record_plans_match_normal_node_and_edge_queries() {
    let (storage, _temp) = create_test_storage();
    let plans = compiled_taxonomy_plans();
    assert!(storage.write_record_plans(&plans).unwrap() >= 10);

    assert_eq!(
        storage
            .get_nodes_by_property("rank", &Value::String("genus".to_string()))
            .unwrap()[0]
            .id,
        "parent"
    );
    assert_eq!(
        storage
            .get_outgoing_edges("child", Some("taxonomy_parent"))
            .unwrap()[0]
            .target,
        "parent"
    );
}

fn compiled_taxonomy_plans() -> [StorageRecordPlan; 3] {
    let mut parent = create_test_node("parent");
    parent
        .properties
        .insert("rank".to_string(), Value::String("genus".to_string()));
    let edge = Edge::with_id(
        "child:taxonomy_parent:parent".to_string(),
        "taxonomy_parent".to_string(),
        "child".to_string(),
        "parent".to_string(),
        HashMap::new(),
    );
    [
        Storage::compile_node_records(&parent).unwrap(),
        Storage::compile_node_records(&create_test_node("child")).unwrap(),
        Storage::compile_edge_records(&edge).unwrap(),
    ]
}

#[test]
fn test_create_and_get_memory() {
    let (storage, _temp) = create_test_storage();

    let mut content = HashMap::new();
    content.insert("message".to_string(), Value::String("Hello".to_string()));

    let memory = storage
        .create_memory("mem1", "conversation", content, 0.8)
        .unwrap();
    assert!(memory.labels.contains(&"memory".to_string()));
    assert!(memory.labels.contains(&"conversation".to_string()));

    let retrieved = storage.get_memory("mem1").unwrap();
    assert!(retrieved.is_some());
}

#[test]
fn test_query_memories_by_type() {
    let (storage, _temp) = create_test_storage();

    let mut content1 = HashMap::new();
    content1.insert("message".to_string(), Value::String("Hello".to_string()));

    let mut content2 = HashMap::new();
    content2.insert("message".to_string(), Value::String("World".to_string()));

    storage
        .create_memory("mem1", "conversation", content1, 0.8)
        .unwrap();
    storage
        .create_memory("mem2", "knowledge", content2, 0.9)
        .unwrap();

    let conversations = storage.query_memories_by_type("conversation").unwrap();
    assert_eq!(conversations.len(), 1);

    let knowledge = storage.query_memories_by_type("knowledge").unwrap();
    assert_eq!(knowledge.len(), 1);
}
