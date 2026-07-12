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
    #[serde(rename = "patch_node_properties")]
    PatchNodeProperties {
        path: Option<String>,
        id: String,
        #[serde(default)]
        set: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        remove: Vec<String>,
    },
    #[serde(rename = "set_node_property")]
    SetNodeProperty {
        path: Option<String>,
        id: String,
        key: String,
        value: serde_json::Value,
    },
    #[serde(rename = "remove_node_property")]
    RemoveNodeProperty {
        path: Option<String>,
        id: String,
        key: String,
    },
    #[serde(rename = "patch_edge_properties")]
    PatchEdgeProperties {
        path: Option<String>,
        id: String,
        #[serde(default)]
        set: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        remove: Vec<String>,
    },
    #[serde(rename = "set_edge_property")]
    SetEdgeProperty {
        path: Option<String>,
        id: String,
        key: String,
        value: serde_json::Value,
    },
    #[serde(rename = "remove_edge_property")]
    RemoveEdgeProperty {
        path: Option<String>,
        id: String,
        key: String,
    },
    #[serde(rename = "edge_ids")]
    EdgeIds {
        path: Option<String>,
        source: String,
        edge_type: String,
        target: String,
    },
    #[serde(rename = "put_index_profile")]
    PutIndexProfile {
        path: Option<String>,
        profile: zlf_index::IndexProfileArtifact,
    },
    #[serde(rename = "activate_index_profile")]
    ActivateIndexProfile {
        path: Option<String>,
        name: String,
        version: u32,
    },
    #[serde(rename = "list_index_profiles")]
    ListIndexProfiles { path: Option<String> },
    #[serde(rename = "index_status")]
    IndexStatus {
        path: Option<String>,
        target: String,
    },
    #[serde(rename = "wait_indexes")]
    WaitIndexes {
        path: Option<String>,
        targets: Vec<String>,
        minimum_sequence: u64,
        timeout_ms: u64,
    },
    #[serde(rename = "query")]
    Query { path: Option<String>, query: String },
    #[serde(rename = "search")]
    Search { path: Option<String>, query: String },
    #[serde(rename = "import")]
    Import { path: Option<String>, file: String },
    #[serde(rename = "export")]
    Export {
        path: Option<String>,
        file: Option<String>,
    },
    #[serde(rename = "embed")]
    Embed {
        text: String,
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

impl Request {
    pub(crate) fn is_extended(&self) -> bool {
        matches!(
            self,
            Self::PatchNodeProperties { .. }
                | Self::PatchEdgeProperties { .. }
                | Self::SetNodeProperty { .. }
                | Self::SetEdgeProperty { .. }
                | Self::RemoveNodeProperty { .. }
                | Self::RemoveEdgeProperty { .. }
                | Self::EdgeIds { .. }
                | Self::PutIndexProfile { .. }
                | Self::ActivateIndexProfile { .. }
                | Self::ListIndexProfiles { .. }
                | Self::IndexStatus { .. }
                | Self::WaitIndexes { .. }
        )
    }
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub(crate) enum Response {
    #[serde(rename = "success")]
    Success { data: serde_json::Value },
    #[serde(rename = "error")]
    Error { code: String, message: String },
}
