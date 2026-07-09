use super::error::{WamError, WamResult};
use super::storage_index_writer::Embedder;
use zlf_embed::{create_provider, EmbeddingConfig, EmbeddingProvider, ProviderType};

pub struct BlockingEmbeddingProvider {
    model: String,
    provider: Box<dyn EmbeddingProvider>,
    runtime: tokio::runtime::Runtime,
}

impl BlockingEmbeddingProvider {
    pub fn new(config: EmbeddingConfig) -> WamResult<Self> {
        let model = config.model.clone();
        let provider = create_provider(config);
        let runtime = tokio::runtime::Runtime::new().map_err(provider_error)?;
        Ok(Self {
            model,
            provider,
            runtime,
        })
    }

    pub fn ollama_bge_m3(endpoint: impl Into<String>) -> WamResult<Self> {
        Self::new(EmbeddingConfig {
            provider: ProviderType::Ollama,
            api_endpoint: endpoint.into(),
            api_key: None,
            model: "bge-m3:latest".to_string(),
            dimension: 1024,
        })
    }
}

impl Embedder for BlockingEmbeddingProvider {
    fn model(&self) -> &str {
        &self.model
    }

    fn embed(&self, text: &str) -> WamResult<Vec<f32>> {
        self.runtime
            .block_on(self.provider.embed(text))
            .map_err(provider_error)
    }
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
