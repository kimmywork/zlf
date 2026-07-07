use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use zlf_core::{Node, Edge};
use zlf_query::QueryPlanner;
use zlf_embed::{EmbeddingConfig, ProviderType, create_provider};
use zlf_config::ZlfConfig;
use axum::{Router, routing::post, Json, response::IntoResponse, extract::State};
use tower_http::cors::{CorsLayer, Any};

// Shared application state
struct AppState {
    db_path: RwLock<String>,
    db: RwLock<Option<Arc<QueryPlanner>>>,
}

#[derive(Deserialize)]
#[serde(tag = "command")]
enum Request {
    #[serde(rename = "init")]
    Init { path: Option<String> },
    #[serde(rename = "add_node")]
    AddNode { path: Option<String>, labels: Vec<String>, properties: serde_json::Value },
    #[serde(rename = "get_node")]
    GetNode { path: Option<String>, id: String },
    #[serde(rename = "add_edge")]
    AddEdge { path: Option<String>, edge_type: String, source: String, target: String, properties: serde_json::Value },
    #[serde(rename = "get_edge")]
    GetEdge { path: Option<String>, id: String },
    #[serde(rename = "query")]
    Query { path: Option<String>, query: String },
    #[serde(rename = "search")]
    Search { path: Option<String>, query: String },
    #[serde(rename = "similar")]
    Similar { path: Option<String>, node_id: String, threshold: f32, limit: usize },
    #[serde(rename = "import")]
    Import { path: Option<String>, file: String },
    #[serde(rename = "export")]
    Export { path: Option<String>, file: Option<String> },
    #[serde(rename = "index_text")]
    IndexText { path: Option<String>, node_id: String, text: String },
    #[serde(rename = "embed")]
    Embed { text: String, #[serde(default)] config: Option<EmbeddingConfig> },
    #[serde(rename = "index_embedding")]
    IndexEmbedding { path: Option<String>, node_id: String, text: String, #[serde(default)] config: Option<EmbeddingConfig> },
    #[serde(rename = "config")]
    Config { #[serde(default)] set: Option<ZlfConfig>, get: Option<bool> },
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum Response {
    #[serde(rename = "success")]
    Success { data: serde_json::Value },
    #[serde(rename = "error")]
    Error { code: String, message: String },
}

fn json_to_properties(json: serde_json::Value) -> std::collections::HashMap<String, zlf_core::Value> {
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
            zlf_core::Value::Array(arr.iter().map(|v| json_to_value(v)).collect())
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

async fn ensure_db(state: &AppState, path: &str) -> Result<Arc<QueryPlanner>, String> {
    // Check if we need to open a new database
    {
        let db_path = state.db_path.read().await;
        let db = state.db.read().await;
        
        if *db_path == path {
            if let Some(planner) = db.as_ref() {
                return Ok(Arc::clone(planner));
            }
        }
    }
    
    // Open new database
    let db_path = std::path::Path::new(path);
    let planner = if db_path.exists() {
        QueryPlanner::open_existing(db_path)
    } else {
        std::fs::create_dir_all(db_path).map_err(|e| e.to_string())?;
        QueryPlanner::open(db_path)
    }.map_err(|e| e.to_string())?;
    
    let planner = Arc::new(planner);
    
    // Update state
    {
        let mut db_path = state.db_path.write().await;
        let mut db = state.db.write().await;
        *db_path = path.to_string();
        *db = Some(Arc::clone(&planner));
    }
    
    Ok(planner)
}

async fn handle_request(request: Request, state: &AppState) -> Response {
    let config = ZlfConfig::load();
    
    match request {
        Request::Init { path } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            let db_path = std::path::Path::new(&path);
            
            if db_path.exists() {
                return Response::Error { 
                    code: "INIT_FAILED".to_string(), 
                    message: format!("Database already exists: {}", path) 
                };
            }
            
            match std::fs::create_dir_all(db_path) {
                Ok(_) => {
                    match QueryPlanner::open(db_path) {
                        Ok(_) => Response::Success { data: serde_json::json!({ "path": path }) },
                        Err(e) => Response::Error { code: "INIT_FAILED".to_string(), message: e.to_string() },
                    }
                }
                Err(e) => Response::Error { code: "INIT_FAILED".to_string(), message: e.to_string() },
            }
        }
        Request::AddNode { path, labels, properties } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    let props = json_to_properties(properties);
                    let node = Node::new(labels, props);
                    match db.add_node(node) {
                        Ok(node) => Response::Success { data: serde_json::to_value(node).unwrap() },
                        Err(e) => Response::Error { code: "ADD_NODE_FAILED".to_string(), message: e.to_string() },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::GetNode { path, id } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    match db.get_node(&id) {
                        Ok(Some(node)) => Response::Success { data: serde_json::to_value(node).unwrap() },
                        Ok(None) => Response::Error { code: "NODE_NOT_FOUND".to_string(), message: format!("Node {} not found", id) },
                        Err(e) => Response::Error { code: "GET_NODE_FAILED".to_string(), message: e.to_string() },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::AddEdge { path, edge_type, source, target, properties } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    let props = json_to_properties(properties);
                    let edge = Edge::new(edge_type, source, target, props);
                    match db.add_edge(edge) {
                        Ok(edge) => Response::Success { data: serde_json::to_value(edge).unwrap() },
                        Err(e) => Response::Error { code: "ADD_EDGE_FAILED".to_string(), message: e.to_string() },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::GetEdge { path, id } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    match db.get_edge(&id) {
                        Ok(Some(edge)) => Response::Success { data: serde_json::to_value(edge).unwrap() },
                        Ok(None) => Response::Error { code: "EDGE_NOT_FOUND".to_string(), message: format!("Edge {} not found", id) },
                        Err(e) => Response::Error { code: "GET_EDGE_FAILED".to_string(), message: e.to_string() },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::Query { path, query } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    match db.execute(&query) {
                        Ok(results) => Response::Success { data: serde_json::json!(results) },
                        Err(e) => Response::Error { code: "QUERY_FAILED".to_string(), message: e.to_string() },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::Search { path, query } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    match db.search(&query) {
                        Ok(results) => {
                            let data: Vec<_> = results.into_iter().map(|(id, score)| {
                                serde_json::json!({ "node_id": id, "score": score })
                            }).collect();
                            Response::Success { data: serde_json::json!(data) }
                        }
                        Err(e) => Response::Error { code: "SEARCH_FAILED".to_string(), message: e.to_string() },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::Similar { path, node_id, threshold, limit } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    match db.similar(&node_id, threshold, limit) {
                        Ok(results) => {
                            let data: Vec<_> = results.into_iter().map(|(id, similarity)| {
                                serde_json::json!({ "node_id": id, "similarity": similarity })
                            }).collect();
                            Response::Success { data: serde_json::json!(data) }
                        }
                        Err(e) => Response::Error { code: "SIMILAR_FAILED".to_string(), message: e.to_string() },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::Import { path, file } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    match import_json(&db, &file) {
                        Ok(count) => Response::Success { data: serde_json::json!({ "imported": count }) },
                        Err(e) => Response::Error { code: "IMPORT_FAILED".to_string(), message: e },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::Export { path, file } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    match export_json(&db, file.as_deref()) {
                        Ok(data) => Response::Success { data },
                        Err(e) => Response::Error { code: "EXPORT_FAILED".to_string(), message: e },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::IndexText { path, node_id, text } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            match ensure_db(&state, &path).await {
                Ok(db) => {
                    match db.index_text(&node_id, &text) {
                        Ok(_) => Response::Success { data: serde_json::json!({ "indexed": true }) },
                        Err(e) => Response::Error { code: "INDEX_FAILED".to_string(), message: e.to_string() },
                    }
                }
                Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
            }
        }
        Request::Embed { text, config: embed_config } => {
            let embed_config = embed_config.unwrap_or_else(|| config.to_embed_config());
            let provider = create_provider(embed_config);
            match provider.embed(&text).await {
                Ok(embedding) => Response::Success { data: serde_json::json!({ "embedding": embedding }) },
                Err(e) => Response::Error { code: "EMBED_FAILED".to_string(), message: e.to_string() },
            }
        }
        Request::IndexEmbedding { path, node_id, text, config: embed_config } => {
            let path = path.unwrap_or_else(|| config.db_path.clone());
            let embed_config = embed_config.unwrap_or_else(|| config.to_embed_config());
            let provider_name = match embed_config.provider {
                ProviderType::Ollama => "ollama",
                ProviderType::OpenAI => "openai",
                ProviderType::HuggingFace => "huggingface",
            }.to_string();
            
            let provider = create_provider(embed_config);
            match provider.embed(&text).await {
                Ok(embedding) => {
                    match ensure_db(&state, &path).await {
                        Ok(db) => {
                            match db.index_embedding(&node_id, &embedding, &provider_name) {
                                Ok(_) => Response::Success { data: serde_json::json!({ "indexed": true, "dimension": embedding.len() }) },
                                Err(e) => Response::Error { code: "INDEX_FAILED".to_string(), message: e.to_string() },
                            }
                        }
                        Err(e) => Response::Error { code: "DB_OPEN_FAILED".to_string(), message: e },
                    }
                }
                Err(e) => Response::Error { code: "EMBED_FAILED".to_string(), message: e.to_string() },
            }
        }
        Request::Config { set, get } => {
            if let Some(new_config) = set {
                match new_config.save(None) {
                    Ok(_) => Response::Success { data: serde_json::json!({ "saved": true }) },
                    Err(e) => Response::Error { code: "CONFIG_SAVE_FAILED".to_string(), message: e },
                }
            } else if get.unwrap_or(true) {
                Response::Success { data: serde_json::to_value(&config).unwrap() }
            } else {
                Response::Error { code: "INVALID_REQUEST".to_string(), message: "Specify set or get".to_string() }
            }
        }
    }
}

fn import_json(db: &QueryPlanner, file: &str) -> Result<usize, String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON: {}", e))?;
    
    let mut count = 0;
    
    if let Some(nodes) = data.get("nodes").and_then(|n| n.as_array()) {
        for node_data in nodes {
            let labels = node_data.get("labels")
                .and_then(|l| l.as_array())
                .map(|l| l.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            
            let properties = node_data.get("properties")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            
            let node = Node::new(labels, json_to_properties(properties));
            match db.add_node(node) {
                Ok(_) => count += 1,
                Err(e) => eprintln!("Warning: Failed to import node: {}", e),
            }
        }
    }
    
    if let Some(edges) = data.get("edges").and_then(|e| e.as_array()) {
        for edge_data in edges {
            let edge_type = edge_data.get("edge_type")
                .and_then(|e| e.as_str())
                .unwrap_or("")
                .to_string();
            
            let source = edge_data.get("source")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            
            let target = edge_data.get("target")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            
            let properties = edge_data.get("properties")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            
            let edge = Edge::new(edge_type, source, target, json_to_properties(properties));
            match db.add_edge(edge) {
                Ok(_) => count += 1,
                Err(e) => eprintln!("Warning: Failed to import edge: {}", e),
            }
        }
    }
    
    Ok(count)
}

fn export_json(db: &QueryPlanner, file: Option<&str>) -> Result<serde_json::Value, String> {
    let data = serde_json::json!({
        "nodes": [],
        "edges": []
    });
    
    if let Some(file_path) = file {
        std::fs::write(file_path, serde_json::to_string_pretty(&data).unwrap())
            .map_err(|e| format!("Failed to write file: {}", e))?;
    }
    
    Ok(data)
}

async fn serve_http(port: u16) -> Result<()> {
    let state = Arc::new(AppState {
        db_path: RwLock::new(String::new()),
        db: RwLock::new(None),
    });
    
    let app = Router::new()
        .route("/api", post(handle_http_request))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
        .with_state(state);
    
    let addr = format!("0.0.0.0:{}", port);
    println!("zlf server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn handle_http_request(
    State(state): State<Arc<AppState>>,
    Json(request): Json<Request>,
) -> impl IntoResponse {
    let response = handle_request(request, &state).await;
    Json(response)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 && args[1] == "serve" {
        let port = args.get(2).and_then(|p| p.parse::<u16>().ok()).unwrap_or(8520);
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(serve_http(port))?;
        return Ok(());
    }
    
    // STDIO mode (default)
    let stdin = io::stdin();
    let stdout = io::stdout();
    
    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();
        
        if line.is_empty() {
            continue;
        }
        
        let response = match serde_json::from_str::<Request>(line) {
            Ok(request) => {
                // Create a temporary state for STDIO mode
                let state = AppState {
                    db_path: RwLock::new(String::new()),
                    db: RwLock::new(None),
                };
                // Use a tokio runtime for STDIO mode
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(handle_request(request, &state))
            },
            Err(e) => Response::Error { 
                code: "INVALID_REQUEST".to_string(), 
                message: format!("Invalid JSON: {}", e) 
            },
        };
        
        let mut out = stdout.lock();
        serde_json::to_writer(&mut out, &response)?;
        writeln!(out)?;
        out.flush()?;
    }
    
    Ok(())
}
