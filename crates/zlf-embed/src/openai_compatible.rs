use crate::{EmbedError, EmbeddingConfig, EmbeddingProvider, Result};

struct OpenAICompatibleClient {
    config: EmbeddingConfig,
    client: reqwest::Client,
}

impl OpenAICompatibleClient {
    fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let request = serde_json::json!({
            "model": self.config.model,
            "input": texts,
        });
        let mut builder = self.client.post(self.url()).json(&request);
        if let Some(api_key) = &self.config.api_key {
            builder = builder.header("Authorization", format!("Bearer {api_key}"));
        }
        let response = builder.send().await?.error_for_status()?;
        let value: serde_json::Value = response.json().await?;
        let data = value["data"]
            .as_array()
            .ok_or_else(|| EmbedError::InvalidResponse("missing embedding data".into()))?;
        let mut indexed = data
            .iter()
            .map(|item| {
                let index = item["index"].as_u64().unwrap_or_default() as usize;
                parse_vector(&item["embedding"]).map(|vector| (index, vector))
            })
            .collect::<Result<Vec<_>>>()?;
        indexed.sort_by_key(|(index, _)| *index);
        if indexed.len() != texts.len() {
            return Err(EmbedError::InvalidResponse(
                "embedding batch cardinality mismatch".into(),
            ));
        }
        Ok(indexed.into_iter().map(|(_, vector)| vector).collect())
    }

    fn url(&self) -> String {
        let endpoint = self.config.api_endpoint.trim_end_matches('/');
        if endpoint.ends_with("/v1") {
            format!("{endpoint}/embeddings")
        } else {
            format!("{endpoint}/v1/embeddings")
        }
    }
}

fn parse_vector(value: &serde_json::Value) -> Result<Vec<f32>> {
    value
        .as_array()
        .ok_or_else(|| EmbedError::InvalidResponse("missing embedding vector".into()))?
        .iter()
        .map(|value| {
            value
                .as_f64()
                .map(|number| number as f32)
                .ok_or_else(|| EmbedError::InvalidResponse("non-numeric embedding value".into()))
        })
        .collect()
}

pub struct OllamaProvider(OpenAICompatibleClient);

impl OllamaProvider {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self(OpenAICompatibleClient::new(config))
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_batch(&[text])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| EmbedError::InvalidResponse("missing embedding".into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.0.embed_batch(texts).await
    }

    fn dimension(&self) -> usize {
        self.0.config.dimension
    }

    fn name(&self) -> &str {
        "ollama_openai_compatible"
    }
}

pub struct OpenAIProvider(OpenAICompatibleClient);

impl OpenAIProvider {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self(OpenAICompatibleClient::new(config))
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAIProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_batch(&[text])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| EmbedError::InvalidResponse("missing embedding".into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.0.embed_batch(texts).await
    }

    fn dimension(&self) -> usize {
        self.0.config.dimension
    }

    fn name(&self) -> &str {
        "openai"
    }
}
