use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[allow(clippy::too_many_lines)]
fn json_profile_api_puts_activates_and_lists() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let responses = run(&[
        serde_json::json!({"command":"init","path":path}),
        serde_json::json!({
            "command":"put_index_profile","path":path,
            "profile": {
                "schema_version":1,"name":"knowledge","version":1,
                "source_hash":"",
                "matcher":{"node_labels":{"labels":["document"]}},
                "fields":{"body":{"bm25":{
                    "analyzer_id":"unicode_jieba_v1","analyzer_version":1,
                    "weight":1.0,"k1":1.2,"b":0.75
                }}},
                "created_at":"2026-07-11T00:00:00Z"
            }
        }),
        serde_json::json!({
            "command":"activate_index_profile","path":path,
            "name":"knowledge","version":1
        }),
        serde_json::json!({"command":"list_index_profiles","path":path}),
        serde_json::json!({"command":"index_status","path":path,"target":"bm25"}),
        serde_json::json!({
            "command":"wait_indexes","path":path,"targets":["bm25"],
            "minimum_sequence":1,"timeout_ms":0
        }),
    ]);
    assert_eq!(responses[1]["type"], "success");
    assert_eq!(responses[2]["type"], "success");
    assert_eq!(responses[3]["data"]["profiles"][0]["name"], "knowledge");
    assert_eq!(responses[4]["data"]["target"], "bm25");
    assert_eq!(responses[5]["data"]["reached"], true);
}

fn run(requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_zlf"))
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
