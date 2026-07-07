use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZlfConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,
    
    #[serde(default)]
    pub embedding: EmbeddingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
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

impl ZlfConfig {
    pub fn load() -> Self {
        // Try to load from current directory
        let local_config = std::path::Path::new("zlf.json");
        if local_config.exists() {
            if let Ok(content) = std::fs::read_to_string(local_config) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        
        // Try to load from home directory
        if let Some(home) = dirs::home_dir() {
            let global_config = home.join(".zlf").join("config.json");
            if global_config.exists() {
                if let Ok(content) = std::fs::read_to_string(global_config) {
                    if let Ok(config) = serde_json::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        
        // Return default config
        Self::default()
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
