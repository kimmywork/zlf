use zlf_config::ZlfConfig;
use zlf_core::{Edge, Node};
use zlf_query::ZlfDatabase;

use crate::embed_commands::{
    handle_config, handle_embed, handle_index_embedding, IndexEmbeddingRequest,
};
use crate::io_data::{export_json, import_json};
use crate::protocol::{Request, Response};
use crate::retract_handler;
use crate::state::{ensure_db, AppState};
use crate::values::json_to_properties;

#[allow(clippy::too_many_lines)]
pub(crate) async fn handle_request(request: Request, state: &AppState) -> Response {
    let config = ZlfConfig::load();

    match request {
        Request::Init { path } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            let db_path = std::path::Path::new(&path);

            if db_path.exists() {
                return Response::Error {
                    code: "INIT_FAILED".to_string(),
                    message: format!("Database already exists: {}", path),
                };
            }

            match std::fs::create_dir_all(db_path) {
                Ok(_) => match ZlfDatabase::open(db_path) {
                    Ok(_) => Response::Success {
                        data: serde_json::json!({ "path": path }),
                    },
                    Err(e) => Response::Error {
                        code: "INIT_FAILED".to_string(),
                        message: e.to_string(),
                    },
                },
                Err(e) => Response::Error {
                    code: "INIT_FAILED".to_string(),
                    message: e.to_string(),
                },
            }
        }
        Request::AddNode {
            path,
            labels,
            properties,
        } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => {
                    let props = json_to_properties(properties);
                    let node = Node::new(labels, props);
                    match db.add_node(node) {
                        Ok(node) => Response::Success {
                            data: serde_json::to_value(node).unwrap(),
                        },
                        Err(e) => Response::Error {
                            code: "ADD_NODE_FAILED".to_string(),
                            message: e.to_string(),
                        },
                    }
                }
                Err(e) => Response::Error {
                    code: "DB_OPEN_FAILED".to_string(),
                    message: e,
                },
            }
        }
        Request::GetNode { path, id } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => match db.get_node(&id) {
                    Ok(Some(node)) => Response::Success {
                        data: serde_json::to_value(node).unwrap(),
                    },
                    Ok(None) => Response::Error {
                        code: "NODE_NOT_FOUND".to_string(),
                        message: format!("Node {} not found", id),
                    },
                    Err(e) => Response::Error {
                        code: "GET_NODE_FAILED".to_string(),
                        message: e.to_string(),
                    },
                },
                Err(e) => Response::Error {
                    code: "DB_OPEN_FAILED".to_string(),
                    message: e,
                },
            }
        }
        Request::AddEdge {
            path,
            edge_type,
            source,
            target,
            properties,
        } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => {
                    let props = json_to_properties(properties);
                    let edge = Edge::new(edge_type, source, target, props);
                    match db.add_edge(edge) {
                        Ok(edge) => Response::Success {
                            data: serde_json::to_value(edge).unwrap(),
                        },
                        Err(e) => Response::Error {
                            code: "ADD_EDGE_FAILED".to_string(),
                            message: e.to_string(),
                        },
                    }
                }
                Err(e) => Response::Error {
                    code: "DB_OPEN_FAILED".to_string(),
                    message: e,
                },
            }
        }
        Request::GetEdge { path, id } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => match db.get_edge(&id) {
                    Ok(Some(edge)) => Response::Success {
                        data: serde_json::to_value(edge).unwrap(),
                    },
                    Ok(None) => Response::Error {
                        code: "EDGE_NOT_FOUND".to_string(),
                        message: format!("Edge {} not found", id),
                    },
                    Err(e) => Response::Error {
                        code: "GET_EDGE_FAILED".to_string(),
                        message: e.to_string(),
                    },
                },
                Err(e) => Response::Error {
                    code: "DB_OPEN_FAILED".to_string(),
                    message: e,
                },
            }
        }
        Request::Query { path, query } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => match db.query_prolog(&query) {
                    Ok(results) => Response::Success {
                        data: serde_json::json!(results),
                    },
                    Err(e) => Response::Error {
                        code: "QUERY_FAILED".to_string(),
                        message: e.to_string(),
                    },
                },
                Err(e) => Response::Error {
                    code: "DB_OPEN_FAILED".to_string(),
                    message: e,
                },
            }
        }
        Request::Search { path, query } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => match db.search(&query) {
                    Ok(results) => {
                        let data: Vec<_> = results
                            .into_iter()
                            .map(|(id, score)| serde_json::json!({ "node_id": id, "score": score }))
                            .collect();
                        Response::Success {
                            data: serde_json::json!(data),
                        }
                    }
                    Err(e) => Response::Error {
                        code: "SEARCH_FAILED".to_string(),
                        message: e.to_string(),
                    },
                },
                Err(e) => Response::Error {
                    code: "DB_OPEN_FAILED".to_string(),
                    message: e,
                },
            }
        }
        Request::Similar {
            path,
            node_id,
            threshold,
            limit,
        } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => match db.similar(&node_id, threshold, limit) {
                    Ok(results) => {
                        let data: Vec<_> = results.into_iter().map(|(id, similarity)| {
                                serde_json::json!({ "node_id": id, "similarity": similarity })
                            }).collect();
                        Response::Success {
                            data: serde_json::json!(data),
                        }
                    }
                    Err(e) => Response::Error {
                        code: "SIMILAR_FAILED".to_string(),
                        message: e.to_string(),
                    },
                },
                Err(e) => Response::Error {
                    code: "DB_OPEN_FAILED".to_string(),
                    message: e,
                },
            }
        }
        Request::Import { path, file } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => match import_json(&db, &file) {
                    Ok(count) => Response::Success {
                        data: serde_json::json!({ "imported": count }),
                    },
                    Err(e) => Response::Error {
                        code: "IMPORT_FAILED".to_string(),
                        message: e,
                    },
                },
                Err(e) => Response::Error {
                    code: "DB_OPEN_FAILED".to_string(),
                    message: e,
                },
            }
        }
        Request::Export { path, file } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => match export_json(&db, file.as_deref()) {
                    Ok(data) => Response::Success { data },
                    Err(e) => Response::Error {
                        code: "EXPORT_FAILED".to_string(),
                        message: e,
                    },
                },
                Err(e) => Response::Error {
                    code: "DB_OPEN_FAILED".to_string(),
                    message: e,
                },
            }
        }
        Request::IndexText {
            path,
            node_id,
            text,
        } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(state, &path).await {
                Ok(db) => match db.index_text(&node_id, &text) {
                    Ok(_) => Response::Success {
                        data: serde_json::json!({ "indexed": true }),
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
            }
        }
        Request::Embed {
            text,
            config: embed_config,
        } => handle_embed(text, embed_config, &config).await,
        Request::IndexEmbedding {
            path,
            node_id,
            text,
            embedding,
            config: embed_config,
        } => {
            handle_index_embedding(
                IndexEmbeddingRequest {
                    path,
                    node_id,
                    text,
                    embedding,
                    config: embed_config,
                },
                &config,
                state,
            )
            .await
        }
        Request::Retract { path, fact } => {
            retract_handler::handle_retract(path, fact, &config, state).await
        }
        Request::Config { set, get } => handle_config(set, get, &config),
    }
}
