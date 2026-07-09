use std::collections::HashMap;
use zlf_core::{Edge, Node, Value};
use zlf_query::ZlfDatabase;

fn main() {
    let test_dir = std::env::temp_dir().join("zlf_example_test");
    let _ = std::fs::remove_dir_all(&test_dir);
    std::fs::create_dir_all(&test_dir).unwrap();

    println!("Creating database at {:?}", test_dir);

    // Create planner
    let planner = ZlfDatabase::open(&test_dir).unwrap();

    // Add nodes
    println!("\nAdding nodes...");

    let mut props1 = HashMap::new();
    props1.insert("name".to_string(), Value::String("Alice".to_string()));
    props1.insert("age".to_string(), Value::Number(30.0));
    let node1 = Node::new(vec!["person".to_string()], props1);
    let alice = planner.add_node(node1).unwrap();
    println!("Created Alice: {}", alice.id);

    let mut props2 = HashMap::new();
    props2.insert("name".to_string(), Value::String("Bob".to_string()));
    props2.insert("age".to_string(), Value::Number(25.0));
    let node2 = Node::new(vec!["person".to_string()], props2);
    let bob = planner.add_node(node2).unwrap();
    println!("Created Bob: {}", bob.id);

    let mut props3 = HashMap::new();
    props3.insert("name".to_string(), Value::String("ACME".to_string()));
    props3.insert("industry".to_string(), Value::String("Tech".to_string()));
    let node3 = Node::new(vec!["company".to_string()], props3);
    let acme = planner.add_node(node3).unwrap();
    println!("Created ACME: {}", acme.id);

    // Add edges
    println!("\nAdding edges...");

    let edge1 = Edge::new(
        "knows".to_string(),
        alice.id.clone(),
        bob.id.clone(),
        HashMap::new(),
    );
    let knows_edge = planner.add_edge(edge1).unwrap();
    println!("Created knows edge: {}", knows_edge.id);

    let mut edge_props = HashMap::new();
    edge_props.insert("role".to_string(), Value::String("Engineer".to_string()));
    let edge2 = Edge::new(
        "works_at".to_string(),
        bob.id.clone(),
        acme.id.clone(),
        edge_props,
    );
    let works_at_edge = planner.add_edge(edge2).unwrap();
    println!("Created works_at edge: {}", works_at_edge.id);

    // Retrieve nodes
    println!("\nRetrieving nodes...");

    let retrieved_alice = planner.get_node(&alice.id).unwrap();
    assert!(retrieved_alice.is_some());
    println!(
        "Retrieved Alice: {:?}",
        retrieved_alice.unwrap().properties.get("name")
    );

    let retrieved_bob = planner.get_node(&bob.id).unwrap();
    assert!(retrieved_bob.is_some());
    println!(
        "Retrieved Bob: {:?}",
        retrieved_bob.unwrap().properties.get("name")
    );

    // Retrieve edge
    println!("\nRetrieving edge...");

    let retrieved_edge = planner.get_edge(&knows_edge.id).unwrap();
    assert!(retrieved_edge.is_some());
    let edge = retrieved_edge.unwrap();
    println!("Retrieved edge: {} -> {}", edge.source, edge.target);

    println!("\n✓ All tests passed!");

    // Cleanup
    let _ = std::fs::remove_dir_all(&test_dir);
}
