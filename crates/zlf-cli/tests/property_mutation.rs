use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn json_api_mutates_edge_properties_and_returns_identity() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let requests = [
        serde_json::json!({"command":"init","path":path}),
        serde_json::json!({"command":"add_node","path":path,"labels":[],"properties":{}}),
    ];
    let first = run(&requests);
    let source = first[1]["data"]["id"].as_str().unwrap().to_string();
    let target_response = run(&[serde_json::json!({
        "command":"add_node","path":path,"labels":[],"properties":{}
    })]);
    let target = target_response[0]["data"]["id"].as_str().unwrap();
    let edge_response = run(&[serde_json::json!({
        "command":"add_edge","path":path,"edge_type":"knows",
        "source":source,"target":target,"properties":{}
    })]);
    let edge_id = edge_response[0]["data"]["id"].as_str().unwrap();
    let responses = run(&[
        serde_json::json!({
            "command":"patch_edge_properties","path":path,"id":edge_id,
            "set":{"confidence":0.9,"nullable":null},"remove":[]
        }),
        serde_json::json!({
            "command":"edge_ids","path":path,"source":source,
            "edge_type":"knows","target":target
        }),
    ]);
    assert_eq!(responses[0]["type"], "success");
    assert_eq!(responses[1]["data"]["edge_ids"][0], edge_id);
}

fn run(requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let executable = env!("CARGO_BIN_EXE_zlf");
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        for request in requests {
            writeln!(stdin, "{request}").unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
