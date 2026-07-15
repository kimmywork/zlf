use zlf_config::ZlfConfig;
use zlf_embed::{create_provider, EmbeddingConfig};

use crate::protocol::Response;

pub(crate) async fn handle_embed(
    text: String,
    embed_config: Option<EmbeddingConfig>,
    config: &ZlfConfig,
) -> Response {
    if !config.embedding.enabled {
        return Response::Error {
            code: "INDEX_UNAVAILABLE".to_string(),
            message: "vector embedding is disabled; set embedding.enabled=true".to_string(),
        };
    }
    let embed_config = embed_config.unwrap_or_else(|| config.to_embed_config());
    let provider = create_provider(embed_config);
    match provider.embed(&text).await {
        Ok(embedding) => Response::Success {
            data: serde_json::json!({ "embedding": embedding }),
        },
        Err(error) => Response::Error {
            code: "EMBED_FAILED".to_string(),
            message: error.to_string(),
        },
    }
}

pub(crate) fn handle_config(
    set: Option<ZlfConfig>,
    get: Option<bool>,
    config: &ZlfConfig,
) -> Response {
    if let Some(new_config) = set {
        match new_config.save(None) {
            Ok(_) => Response::Success {
                data: serde_json::json!({ "saved": true }),
            },
            Err(error) => Response::Error {
                code: "CONFIG_SAVE_FAILED".to_string(),
                message: error,
            },
        }
    } else if get.unwrap_or(true) {
        Response::Success {
            data: serde_json::to_value(config).unwrap(),
        }
    } else {
        Response::Error {
            code: "INVALID_REQUEST".to_string(),
            message: "Specify set or get".to_string(),
        }
    }
}
