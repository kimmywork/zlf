use std::collections::{BTreeMap, BTreeSet};

use zlf_core::PropertyPatch;

use crate::protocol::{Request, Response};
use crate::state::{ensure_db, AppState};
use crate::values::json_to_value;

#[allow(clippy::too_many_lines)]
pub(crate) async fn handle_mutation(
    request: Request,
    default_path: &str,
    state: &AppState,
) -> Response {
    match request {
        Request::PatchNodeProperties {
            path,
            id,
            set,
            remove,
        } => {
            patch_properties(
                state,
                path.as_deref().unwrap_or(default_path),
                &id,
                set,
                remove,
                true,
            )
            .await
        }
        Request::PatchEdgeProperties {
            path,
            id,
            set,
            remove,
        } => {
            patch_properties(
                state,
                path.as_deref().unwrap_or(default_path),
                &id,
                set,
                remove,
                false,
            )
            .await
        }
        Request::SetNodeProperty {
            path,
            id,
            key,
            value,
        } => {
            set_property(
                state,
                path.as_deref().unwrap_or(default_path),
                &id,
                &key,
                value,
                true,
            )
            .await
        }
        Request::SetEdgeProperty {
            path,
            id,
            key,
            value,
        } => {
            set_property(
                state,
                path.as_deref().unwrap_or(default_path),
                &id,
                &key,
                value,
                false,
            )
            .await
        }
        Request::RemoveNodeProperty { path, id, key } => {
            remove_property(
                state,
                path.as_deref().unwrap_or(default_path),
                &id,
                &key,
                true,
            )
            .await
        }
        Request::RemoveEdgeProperty { path, id, key } => {
            remove_property(
                state,
                path.as_deref().unwrap_or(default_path),
                &id,
                &key,
                false,
            )
            .await
        }
        Request::EdgeIds {
            path,
            source,
            edge_type,
            target,
        } => {
            edge_ids(
                state,
                path.as_deref().unwrap_or(default_path),
                &source,
                &edge_type,
                &target,
            )
            .await
        }
        Request::PutIndexProfile { path, profile } => {
            match ensure_db(state, path.as_deref().unwrap_or(default_path)).await {
                Ok(db) => profile_response(db.put_index_profile(&profile)),
                Err(message) => db_error(message),
            }
        }
        Request::ActivateIndexProfile {
            path,
            name,
            version,
        } => match ensure_db(state, path.as_deref().unwrap_or(default_path)).await {
            Ok(db) => profile_response(db.activate_index_profile(&name, version)),
            Err(message) => db_error(message),
        },
        Request::ListIndexProfiles { path } => {
            match ensure_db(state, path.as_deref().unwrap_or(default_path)).await {
                Ok(db) => match db.index_profiles() {
                    Ok(profiles) => Response::Success {
                        data: serde_json::json!({ "profiles": profiles }),
                    },
                    Err(error) => profile_error(error),
                },
                Err(message) => db_error(message),
            }
        }
        _ => Response::Error {
            code: "INVALID_MUTATION_COMMAND".into(),
            message: "not a mutation command".into(),
        },
    }
}

async fn patch_properties(
    state: &AppState,
    path: &str,
    id: &str,
    set: serde_json::Map<String, serde_json::Value>,
    remove: Vec<String>,
    node: bool,
) -> Response {
    let patch = PropertyPatch {
        set: set
            .into_iter()
            .map(|(key, value)| (key, json_to_value(&value)))
            .collect::<BTreeMap<_, _>>(),
        remove: remove.into_iter().collect::<BTreeSet<_>>(),
    };
    match ensure_db(state, path).await {
        Ok(db) => {
            let result = if node {
                db.patch_node_properties(id, &patch)
            } else {
                db.patch_edge_properties(id, &patch)
            };
            mutation_response(result)
        }
        Err(message) => db_error(message),
    }
}

async fn set_property(
    state: &AppState,
    path: &str,
    id: &str,
    key: &str,
    value: serde_json::Value,
    node: bool,
) -> Response {
    match ensure_db(state, path).await {
        Ok(db) => {
            let value = json_to_value(&value);
            let result = if node {
                db.set_node_property(id, key, value)
            } else {
                db.set_edge_property(id, key, value)
            };
            mutation_response(result)
        }
        Err(message) => db_error(message),
    }
}

async fn remove_property(
    state: &AppState,
    path: &str,
    id: &str,
    key: &str,
    node: bool,
) -> Response {
    match ensure_db(state, path).await {
        Ok(db) => {
            let result = if node {
                db.remove_node_property(id, key)
            } else {
                db.remove_edge_property(id, key)
            };
            mutation_response(result)
        }
        Err(message) => db_error(message),
    }
}

async fn edge_ids(
    state: &AppState,
    path: &str,
    source: &str,
    edge_type: &str,
    target: &str,
) -> Response {
    match ensure_db(state, path).await {
        Ok(db) => match db.get_edge_ids(source, edge_type, target) {
            Ok(ids) => Response::Success {
                data: serde_json::json!({ "edge_ids": ids }),
            },
            Err(error) => Response::Error {
                code: "EDGE_LOOKUP_FAILED".into(),
                message: error.to_string(),
            },
        },
        Err(message) => db_error(message),
    }
}

fn mutation_response(result: zlf_core::Result<zlf_storage::MutationReceipt>) -> Response {
    match result {
        Ok(receipt) => Response::Success {
            data: serde_json::to_value(receipt).unwrap_or_default(),
        },
        Err(error) => Response::Error {
            code: "PROPERTY_MUTATION_FAILED".into(),
            message: error.to_string(),
        },
    }
}

fn profile_response(result: zlf_core::Result<u64>) -> Response {
    match result {
        Ok(sequence) => Response::Success {
            data: serde_json::json!({ "sequence": sequence }),
        },
        Err(error) => profile_error(error),
    }
}

fn profile_error(error: zlf_core::ZlfError) -> Response {
    Response::Error {
        code: "INDEX_PROFILE_FAILED".into(),
        message: error.to_string(),
    }
}

fn db_error(message: String) -> Response {
    Response::Error {
        code: "DB_OPEN_FAILED".into(),
        message,
    }
}
