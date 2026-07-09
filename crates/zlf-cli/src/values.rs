pub(crate) fn json_to_properties(
    json: serde_json::Value,
) -> std::collections::HashMap<String, zlf_core::Value> {
    let mut props = std::collections::HashMap::new();

    if let Some(obj) = json.as_object() {
        for (k, v) in obj {
            props.insert(k.clone(), json_to_value(v));
        }
    }

    props
}

fn json_to_value(json: &serde_json::Value) -> zlf_core::Value {
    match json {
        serde_json::Value::Null => zlf_core::Value::Null,
        serde_json::Value::Bool(b) => zlf_core::Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                zlf_core::Value::Number(f)
            } else {
                zlf_core::Value::Null
            }
        }
        serde_json::Value::String(s) => zlf_core::Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            zlf_core::Value::Array(arr.iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = std::collections::HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_value(v));
            }
            zlf_core::Value::Object(map)
        }
    }
}
