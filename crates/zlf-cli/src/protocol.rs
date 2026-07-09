use serde::{Deserialize, Serialize};
use zlf_config::ZlfConfig;
use zlf_embed::EmbeddingConfig;

#[derive(Deserialize)]
#[serde(tag = "command")]
pub(crate) enum Request {
    #[serde(rename = "init")]
    Init { path: Option<String> },
    #[serde(rename = "add_node")]
    AddNode {
        path: Option<String>,
        labels: Vec<String>,
        properties: serde_json::Value,
    },
    #[serde(rename = "get_node")]
    GetNode { path: Option<String>, id: String },
    #[serde(rename = "add_edge")]
    AddEdge {
        path: Option<String>,
        edge_type: String,
        source: String,
        target: String,
        properties: serde_json::Value,
    },
    #[serde(rename = "get_edge")]
    GetEdge { path: Option<String>, id: String },
    #[serde(rename = "query")]
    Query { path: Option<String>, query: String },
    #[serde(rename = "search")]
    Search { path: Option<String>, query: String },
    #[serde(rename = "similar")]
    Similar {
        path: Option<String>,
        node_id: String,
        threshold: f32,
        limit: usize,
    },
    #[serde(rename = "import")]
    Import { path: Option<String>, file: String },
    #[serde(rename = "export")]
    Export {
        path: Option<String>,
        file: Option<String>,
    },
    #[serde(rename = "index_text")]
    IndexText {
        path: Option<String>,
        node_id: String,
        text: String,
    },
    #[serde(rename = "embed")]
    Embed {
        text: String,
        #[serde(default)]
        config: Option<EmbeddingConfig>,
    },
    #[serde(rename = "index_embedding")]
    IndexEmbedding {
        path: Option<String>,
        node_id: String,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        embedding: Option<Vec<f32>>,
        #[serde(default)]
        config: Option<EmbeddingConfig>,
    },
    #[serde(rename = "config")]
    Config {
        #[serde(default)]
        set: Option<ZlfConfig>,
        get: Option<bool>,
    },
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub(crate) enum Response {
    #[serde(rename = "success")]
    Success { data: serde_json::Value },
    #[serde(rename = "error")]
    Error { code: String, message: String },
}
