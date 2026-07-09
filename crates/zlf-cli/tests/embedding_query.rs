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
        stdin.write_all(request.as_bytes()).unwrap();
        stdin.flush().unwrap();
    }

    let output = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_stdio_embedding_graph_composite_query() {
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
    run_zlf_command(&import_request);

    let index_alice = format!(
        r#"{{"command":"index_embedding","path":"{}","node_id":"alice","embedding":[1.0,0.0,0.0]}}"#,
        db_path.display()
    );
    let (stdout, _, success) = run_zlf_command(&index_alice);
    assert!(success, "Index alice embedding should succeed");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).unwrap()["type"],
        "success"
    );

    let index_bob = format!(
        r#"{{"command":"index_embedding","path":"{}","node_id":"bob","embedding":[0.95,0.05,0.0]}}"#,
        db_path.display()
    );
    let (stdout, _, success) = run_zlf_command(&index_bob);
    assert!(success, "Index bob embedding should succeed");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).unwrap()["type"],
        "success"
    );

    let query_request = format!(
        r#"{{"command":"query","path":"{}","query":"?knows(alice, X), vector_similar(alice, X, Score)."}}"#,
        db_path.display()
    );
    let (stdout, _, success) = run_zlf_command(&query_request);
    assert!(success, "Composite embedding query should succeed");
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["type"], "success");
    assert_eq!(response["data"].as_array().unwrap().len(), 1);
    assert_eq!(response["data"][0]["X"], "bob");
}
