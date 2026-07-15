use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZlfConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,

    #[serde(default)]
    pub embedding: EmbeddingConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VectorIndexEngine {
    #[default]
    Exact,
    Hnsw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub index_engine: VectorIndexEngine,

    #[serde(default = "default_provider")]
    pub provider: String,

    #[serde(default = "default_api_endpoint")]
    pub api_endpoint: String,

    #[serde(default)]
    pub api_key: Option<String>,

    #[serde(default = "default_model")]
    pub model: String,

    #[serde(default = "default_dimension")]
    pub dimension: usize,
}

fn default_db_path() -> String {
    "./zlf-db".to_string()
}

fn default_provider() -> String {
    "ollama".to_string()
}

fn default_api_endpoint() -> String {
    "http://localhost:11434".to_string()
}

fn default_model() -> String {
    "bge-m3:latest".to_string()
}

fn default_dimension() -> usize {
    1024
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            index_engine: VectorIndexEngine::Exact,
            provider: default_provider(),
            api_endpoint: default_api_endpoint(),
            api_key: None,
            model: default_model(),
            dimension: default_dimension(),
        }
    }
}

impl Default for ZlfConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            embedding: EmbeddingConfig::default(),
        }
    }
}

fn with_env_overrides(mut config: ZlfConfig) -> ZlfConfig {
    config.apply_env_overrides();
    config
}

impl ZlfConfig {
    pub fn load() -> Self {
        // Try to load from current directory
        let local_config = std::path::Path::new("zlf.json");
        if local_config.exists() {
            if let Ok(content) = std::fs::read_to_string(local_config) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return with_env_overrides(config);
                }
            }
        }

        // Try to load from home directory
        if let Some(home) = dirs::home_dir() {
            let global_config = home.join(".zlf").join("config.json");
            if global_config.exists() {
                if let Ok(content) = std::fs::read_to_string(global_config) {
                    if let Ok(config) = serde_json::from_str(&content) {
                        return with_env_overrides(config);
                    }
                }
            }
        }

        with_env_overrides(Self::default())
    }

    #[allow(clippy::too_many_lines)]
    fn apply_env_overrides(&mut self) {
        if let Ok(value) = std::env::var("ZLF_DB_PATH") {
            self.db_path = value;
        }
        if let Ok(value) = std::env::var("ZLF_EMBED_ENABLED") {
            if let Ok(enabled) = value.parse() {
                self.embedding.enabled = enabled;
            }
        }
        if let Ok(value) = std::env::var("ZLF_VECTOR_INDEX_ENGINE") {
            self.embedding.index_engine = match value.as_str() {
                "hnsw" => VectorIndexEngine::Hnsw,
                _ => VectorIndexEngine::Exact,
            };
        }
        if let Ok(value) = std::env::var("ZLF_EMBED_PROVIDER") {
            self.embedding.provider = value;
        }
        if let Ok(value) = std::env::var("ZLF_EMBED_ENDPOINT") {
            self.embedding.api_endpoint = value;
        } else if let Ok(value) = std::env::var("OLLAMA_ENDPOINT") {
            self.embedding.api_endpoint = value;
        }
        if let Ok(value) = std::env::var("ZLF_EMBED_MODEL") {
            self.embedding.model = value;
        }
        if let Ok(value) = std::env::var("ZLF_EMBED_DIMENSION") {
            if let Ok(dimension) = value.parse() {
                self.embedding.dimension = dimension;
            }
        }
        if let Ok(value) = std::env::var("ZLF_EMBED_API_KEY") {
            self.embedding.api_key = Some(value);
        }
    }

    pub fn save(&self, path: Option<&str>) -> Result<(), String> {
        let config_path = if let Some(p) = path {
            PathBuf::from(p)
        } else {
            std::path::Path::new("zlf.json").to_path_buf()
        };

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(())
    }

    pub fn to_embed_config(&self) -> zlf_embed::EmbeddingConfig {
        zlf_embed::EmbeddingConfig {
            provider: match self.embedding.provider.as_str() {
                "ollama" => zlf_embed::ProviderType::Ollama,
                "openai" => zlf_embed::ProviderType::OpenAI,
                "huggingface" => zlf_embed::ProviderType::HuggingFace,
                _ => zlf_embed::ProviderType::Ollama,
            },
            api_endpoint: self.embedding.api_endpoint.clone(),
            api_key: self.embedding.api_key.clone(),
            model: self.embedding.model.clone(),
            dimension: self.embedding.dimension,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_is_disabled_with_exact_as_dormant_default() {
        let config = ZlfConfig::default();
        assert!(!config.embedding.enabled);
        assert_eq!(config.embedding.index_engine, VectorIndexEngine::Exact);
    }

    #[test]
    fn hnsw_requires_explicit_enablement_and_engine_selection() {
        let config: ZlfConfig =
            serde_json::from_str(r#"{"embedding":{"enabled":true,"index_engine":"hnsw"}}"#)
                .unwrap();
        assert!(config.embedding.enabled);
        assert_eq!(config.embedding.index_engine, VectorIndexEngine::Hnsw);
        assert_eq!(config.embedding.dimension, 1024);
    }
}
