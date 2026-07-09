use zlf_config::ZlfConfig;

use crate::protocol::Response;
use crate::state::ensure_db;

/// Handle a Retract request.
pub(crate) async fn handle_retract(
    path: Option<String>,
    fact: String,
    config: &ZlfConfig,
    state: &crate::state::AppState,
) -> Response {
    let path = path.unwrap_or_else(|| config.db_path.clone());
    match ensure_db(state, &path).await {
        Ok(db) => match db.retract_fact(&fact) {
            Ok(Some(key)) => Response::Success {
                data: serde_json::json!({ "retracted": true, "key": format!("{key:?}") }),
            },
            Ok(None) => Response::Error {
                code: "NOT_FOUND".to_string(),
                message: "No matching fact found to retract".to_string(),
            },
            Err(e) => Response::Error {
                code: "RETRACT_FAILED".to_string(),
                message: e.to_string(),
            },
        },
        Err(e) => Response::Error {
            code: "DB_OPEN_FAILED".to_string(),
            message: e,
        },
    }
}
