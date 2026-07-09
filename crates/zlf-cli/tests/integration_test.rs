#![allow(clippy::needless_borrow)]

use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

fn run_zlf_command(request: &str) -> (String, String, bool) {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "zlf-cli"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn zlf-cli");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(request.as_bytes())
            .expect("Failed to write to stdin");
        stdin.flush().expect("Failed to flush stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait for output");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

#[test]
fn test_init_command() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");
    let request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());

    let (stdout, _, success) = run_zlf_command(&request);
    assert!(success, "Init command should succeed");

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    assert!(db_path.exists(), "Database directory should be created");
}

#[test]
fn test_add_node_command() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");

    // Init first
    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    // Add node
    let add_request = format!(
        r#"{{"command":"add_node","path":"{}","labels":["person"],"properties":{{"name":"Alice","age":30}}}}"#,
        db_path.display()
    );

    let (stdout, _, success) = run_zlf_command(&add_request);
    assert!(success, "Add node command should succeed");

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    assert!(response["data"]["id"].is_string(), "Should return node ID");
    assert_eq!(response["data"]["labels"], serde_json::json!(["person"]));
}

#[test]
fn test_get_node_command() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");

    // Init
    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    // Add node
    let add_request = format!(
        r#"{{"command":"add_node","path":"{}","labels":["person"],"properties":{{"name":"Alice"}}}}"#,
        db_path.display()
    );
    let (stdout, _, _) = run_zlf_command(&add_request);
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let node_id = response["data"]["id"].as_str().unwrap();

    // Get node
    let get_request = format!(
        r#"{{"command":"get_node","path":"{}","id":"{}"}}"#,
        db_path.display(),
        node_id
    );
    let (stdout, _, success) = run_zlf_command(&get_request);
    assert!(success, "Get node command should succeed");

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    assert_eq!(response["data"]["id"], node_id);
}

#[test]
fn test_get_node_not_found() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");

    // Init
    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    // Get non-existent node
    let get_request = format!(
        r#"{{"command":"get_node","path":"{}","id":"nonexistent"}}"#,
        db_path.display()
    );
    let (stdout, _, _) = run_zlf_command(&get_request);

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "error");
    assert_eq!(response["code"], "NODE_NOT_FOUND");
}

#[test]
fn test_add_edge_command() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");

    // Init
    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    // Add nodes
    let add_node1 = format!(
        r#"{{"command":"add_node","path":"{}","labels":["person"],"properties":{{"name":"Alice"}}}}"#,
        db_path.display()
    );
    let (stdout, _, _) = run_zlf_command(&add_node1);
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let node1_id = response["data"]["id"].as_str().unwrap();

    let add_node2 = format!(
        r#"{{"command":"add_node","path":"{}","labels":["person"],"properties":{{"name":"Bob"}}}}"#,
        db_path.display()
    );
    let (stdout, _, _) = run_zlf_command(&add_node2);
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let node2_id = response["data"]["id"].as_str().unwrap();

    // Add edge
    let add_edge = format!(
        r#"{{"command":"add_edge","path":"{}","edge_type":"knows","source":"{}","target":"{}","properties":{{"since":2020}}}}"#,
        db_path.display(),
        node1_id,
        node2_id
    );
    let (stdout, _, success) = run_zlf_command(&add_edge);
    assert!(success, "Add edge command should succeed");

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    assert!(response["data"]["id"].is_string(), "Should return edge ID");
}

#[test]
fn test_invalid_json_input() {
    let (stdout, _, _) = run_zlf_command("invalid json");

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "error");
    assert_eq!(response["code"], "INVALID_REQUEST");
}

#[test]
fn test_empty_database_path() {
    let request = r#"{"command":"get_node","path":"/nonexistent/path","id":"test"}"#;
    let (stdout, _, _) = run_zlf_command(request);

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "error");
    assert_eq!(response["code"], "DB_OPEN_FAILED");
}

#[test]
fn test_multiple_commands_sequentially() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");

    // Init
    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    // Add multiple nodes
    for i in 0..5 {
        let add_request = format!(
            r#"{{"command":"add_node","path":"{}","labels":["person"],"properties":{{"name":"User{}"}}}}"#,
            db_path.display(),
            i
        );
        let (stdout, _, success) = run_zlf_command(&add_request);
        assert!(success, "Add node {} should succeed", i);

        let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(response["type"], "success");
    }
}

#[test]
fn test_special_characters_in_properties() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");

    // Init
    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    // Add node with special characters
    let add_request = format!(
        r#"{{"command":"add_node","path":"{}","labels":["person"],"properties":{{"name":"Alice Smith","bio":"Software engineer with 10+ years experience","emoji":"🎉"}}}}"#,
        db_path.display()
    );
    let (stdout, _, success) = run_zlf_command(&add_request);
    assert!(success, "Add node with special characters should succeed");

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
}

#[test]
fn test_large_properties() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");

    // Init
    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    // Create large property value (>1KB)
    let large_value = "x".repeat(2000);
    let add_request = format!(
        r#"{{"command":"add_node","path":"{}","labels":["document"],"properties":{{"content":"{}"}}}}"#,
        db_path.display(),
        large_value
    );
    let (_stdout, _, success) = run_zlf_command(&add_request);
    assert!(success, "Add node with large properties should succeed");
}

#[test]
fn test_export_empty_database() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");

    // Init
    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    // Export
    let export_request = format!(r#"{{"command":"export","path":"{}"}}"#, db_path.display());
    let (stdout, _, success) = run_zlf_command(&export_request);
    assert!(success, "Export should succeed");

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    assert!(response["data"]["nodes"].is_array());
    assert!(response["data"]["edges"].is_array());
}

#[test]
fn test_import_and_export() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");
    let import_file = temp.path().join("import.json");

    // Init
    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    // Create import file with only nodes (edges require existing nodes)
    let import_data = r#"{
        "nodes": [
            {"labels": ["person"], "properties": {"name": "Alice", "age": 30}},
            {"labels": ["person"], "properties": {"name": "Bob", "age": 25}}
        ],
        "edges": []
    }"#;
    std::fs::write(&import_file, import_data).unwrap();

    // Import
    let import_request = format!(
        r#"{{"command":"import","path":"{}","file":"{}"}}"#,
        db_path.display(),
        import_file.display()
    );
    let (stdout, _, success) = run_zlf_command(&import_request);
    assert!(success, "Import should succeed");

    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    // Should import 2 nodes
    assert_eq!(response["data"]["imported"], 2);
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_import_preserves_ids_and_exports_data() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("test-db");
    let import_file = temp.path().join("kb.json");

    let init_request = format!(r#"{{"command":"init","path":"{}"}}"#, db_path.display());
    run_zlf_command(&init_request);

    let import_data = r#"{
        "nodes": [
            {"id": "alice", "labels": ["person"], "properties": {"name": "Alice"}},
            {"id": "bob", "labels": ["person"], "properties": {"name": "Bob"}}
        ],
        "edges": [
            {"id": "e1", "edge_type": "knows", "source": "alice", "target": "bob", "properties": {}}
        ]
    }"#;
    std::fs::write(&import_file, import_data).unwrap();

    let import_request = format!(
        r#"{{"command":"import","path":"{}","file":"{}"}}"#,
        db_path.display(),
        import_file.display()
    );
    let (stdout, _, success) = run_zlf_command(&import_request);
    assert!(success, "Import should succeed");
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    assert_eq!(response["data"]["imported"], 3);

    let query_request = format!(
        r#"{{"command":"query","path":"{}","query":"?knows(alice, X), property(X, name, \"Bob\")."}}"#,
        db_path.display()
    );
    let (stdout, _, success) = run_zlf_command(&query_request);
    assert!(success, "Composite query should succeed");
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    assert_eq!(response["data"].as_array().unwrap().len(), 1);
    assert_eq!(response["data"][0]["X"], "bob");

    let export_request = format!(r#"{{"command":"export","path":"{}"}}"#, db_path.display());
    let (stdout, _, success) = run_zlf_command(&export_request);
    assert!(success, "Export should succeed");
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    assert_eq!(response["data"]["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(response["data"]["edges"].as_array().unwrap().len(), 1);
}
