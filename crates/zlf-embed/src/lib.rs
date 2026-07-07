use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmbedError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Provider error: {0}")]
    Provider(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub type Result<T> = std::result::Result<T, EmbedError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: ProviderType,
    pub api_endpoint: String,
    #[serde(default)]
    pub api_key: Option<String>,
    pub model: String,
    #[serde(default = "default_dimension")]
    pub dimension: usize,
}

fn default_dimension() -> usize {
    1024
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ProviderType {
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "huggingface")]
    HuggingFace,
}

#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
    fn name(&self) -> &str;
}

pub struct OllamaProvider {
    config: EmbeddingConfig,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = serde_json::json!({
            "model": self.config.model,
            "prompt": text
        });
        
        let response = self.client
            .post(format!("{}/api/embeddings", self.config.api_endpoint))
            .json(&request)
            .send()
            .await?;
        
        let response_json: serde_json::Value = response.json().await?;
        
        response_json["embedding"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            .ok_or_else(|| EmbedError::InvalidResponse("No embedding in response".to_string()))
    }
    
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }
    
    fn dimension(&self) -> usize {
        self.config.dimension
    }
    
    fn name(&self) -> &str {
        "ollama"
    }
}

pub struct OpenAIProvider {
    config: EmbeddingConfig,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAIProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = serde_json::json!({
            "model": self.config.model,
            "input": text
        });
        
        let mut builder = self.client
            .post(format!("{}/v1/embeddings", self.config.api_endpoint))
            .json(&request);
        
        if let Some(api_key) = &self.config.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = builder.send().await?;
        let response_json: serde_json::Value = response.json().await?;
        
        response_json["data"][0]["embedding"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            .ok_or_else(|| EmbedError::InvalidResponse("No embedding in response".to_string()))
    }
    
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let request = serde_json::json!({
            "model": self.config.model,
            "input": texts
        });
        
        let mut builder = self.client
            .post(format!("{}/v1/embeddings", self.config.api_endpoint))
            .json(&request);
        
        if let Some(api_key) = &self.config.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = builder.send().await?;
        let response_json: serde_json::Value = response.json().await?;
        
        response_json["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item["embedding"]
                            .as_array()
                            .map(|emb| emb.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
                    })
                    .collect()
            })
            .ok_or_else(|| EmbedError::InvalidResponse("No embeddings in response".to_string()))
    }
    
    fn dimension(&self) -> usize {
        self.config.dimension
    }
    
    fn name(&self) -> &str {
        "openai"
    }
}

pub struct HuggingFaceProvider {
    config: EmbeddingConfig,
    client: reqwest::Client,
}

impl HuggingFaceProvider {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for HuggingFaceProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = serde_json::json!({
            "inputs": text
        });
        
        let mut builder = self.client
            .post(format!("{}/{}", self.config.api_endpoint, self.config.model))
            .json(&request);
        
        if let Some(api_key) = &self.config.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = builder.send().await?;
        let response_json: serde_json::Value = response.json().await?;
        
        // HuggingFace returns embedding directly or in nested array
        if let Some(arr) = response_json.as_array() {
            if let Some(first) = arr.first() {
                if let Some(emb) = first.as_array() {
                    return Ok(emb.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect());
                }
            }
        }
        
        Err(EmbedError::InvalidResponse("No embedding in response".to_string()))
    }
    
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }
    
    fn dimension(&self) -> usize {
        self.config.dimension
    }
    
    fn name(&self) -> &str {
        "huggingface"
    }
}

pub fn create_provider(config: EmbeddingConfig) -> Box<dyn EmbeddingProvider> {
    match config.provider {
        ProviderType::Ollama => Box::new(OllamaProvider::new(config)),
        ProviderType::OpenAI => Box::new(OpenAIProvider::new(config)),
        ProviderType::HuggingFace => Box::new(HuggingFaceProvider::new(config)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_serialization() {
        let config = EmbeddingConfig {
            provider: ProviderType::Ollama,
            api_endpoint: "http://localhost:11434".to_string(),
            api_key: None,
            model: "bge-m3".to_string(),
            dimension: 1024,
        };
        
        let json = serde_json::to_string(&config).unwrap();
        let parsed: EmbeddingConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.provider, ProviderType::Ollama);
        assert_eq!(parsed.model, "bge-m3");
    }
}
