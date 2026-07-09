use zlf_config::ZlfConfig;
use zlf_embed::{create_provider, EmbeddingConfig, ProviderType};

use crate::protocol::Response;
use crate::state::{ensure_db, AppState};

pub(crate) async fn handle_embed(
    text: String,
    embed_config: Option<EmbeddingConfig>,
    config: &ZlfConfig,
) -> Response {
    let embed_config = embed_config.unwrap_or_else(|| config.to_embed_config());
    let provider = create_provider(embed_config);
    match provider.embed(&text).await {
        Ok(embedding) => Response::Success {
            data: serde_json::json!({ "embedding": embedding }),
        },
        Err(e) => Response::Error {
            code: "EMBED_FAILED".to_string(),
            message: e.to_string(),
        },
    }
}

pub(crate) async fn handle_index_embedding(
    request: IndexEmbeddingRequest,
    config: &ZlfConfig,
    state: &AppState,
) -> Response {
    let path = request.path.unwrap_or_else(|| config.db_path.clone());
    let embed_config = request.config.unwrap_or_else(|| config.to_embed_config());
    let provider_name = provider_name(&embed_config).to_string();
    let embedding_result =
        embedding_from_request(request.text, request.embedding, embed_config).await;

    match embedding_result {
        Ok(embedding) => match ensure_db(state, &path).await {
            Ok(db) => match db.index_embedding(&request.node_id, &embedding, &provider_name) {
                Ok(_) => Response::Success {
                    data: serde_json::json!({ "indexed": true, "dimension": embedding.len() }),
                },
                Err(e) => Response::Error {
                    code: "INDEX_FAILED".to_string(),
                    message: e.to_string(),
                },
            },
            Err(e) => Response::Error {
                code: "DB_OPEN_FAILED".to_string(),
                message: e,
            },
        },
        Err(e) => Response::Error {
            code: "EMBED_FAILED".to_string(),
            message: e,
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
            Err(e) => Response::Error {
                code: "CONFIG_SAVE_FAILED".to_string(),
                message: e,
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

pub(crate) struct IndexEmbeddingRequest {
    pub(crate) path: Option<String>,
    pub(crate) node_id: String,
    pub(crate) text: Option<String>,
    pub(crate) embedding: Option<Vec<f32>>,
    pub(crate) config: Option<EmbeddingConfig>,
}

async fn embedding_from_request(
    text: Option<String>,
    embedding: Option<Vec<f32>>,
    embed_config: EmbeddingConfig,
) -> Result<Vec<f32>, String> {
    if let Some(embedding) = embedding {
        Ok(embedding)
    } else if let Some(text) = text {
        create_provider(embed_config)
            .embed(&text)
            .await
            .map_err(|e| e.to_string())
    } else {
        Err("Specify either embedding or text".to_string())
    }
}

fn provider_name(config: &EmbeddingConfig) -> &'static str {
    match config.provider {
        ProviderType::Ollama => "ollama",
        ProviderType::OpenAI => "openai",
        ProviderType::HuggingFace => "huggingface",
    }
}
