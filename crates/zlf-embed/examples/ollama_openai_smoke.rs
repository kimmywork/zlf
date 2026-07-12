use std::time::Instant;

use zlf_embed::{EmbeddingConfig, EmbeddingProvider, OllamaProvider, ProviderType};

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint =
        std::env::var("OLLAMA_ENDPOINT").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let model = std::env::var("ZLF_EMBED_MODEL").unwrap_or_else(|_| "bge-m3:latest".to_string());
    let provider = OllamaProvider::new(EmbeddingConfig {
        provider: ProviderType::Ollama,
        api_endpoint: endpoint,
        api_key: None,
        model: model.clone(),
        dimension: 1024,
    });
    let inputs = [
        "knowledge graph retrieval",
        "软件工程师",
        "temporal graph database",
        "multilingual semantic search",
    ];
    let characters = inputs
        .iter()
        .map(|text| text.chars().count())
        .sum::<usize>();
    let started = Instant::now();
    let vectors = provider.embed_batch(&inputs).await?;
    let elapsed = started.elapsed();
    if vectors.len() != inputs.len()
        || vectors
            .iter()
            .any(|vector| vector.len() != 1024 || vector.iter().any(|value| !value.is_finite()))
    {
        return Err("invalid Ollama embedding batch".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema":"zlf-ollama-embedding-smoke-v1",
            "provider":provider.name(),
            "model":model,
            "batch_size":inputs.len(),
            "characters":characters,
            "dimension":1024,
            "elapsed_ms":elapsed.as_secs_f64() * 1000.0,
            "documents_per_second":inputs.len() as f64 / elapsed.as_secs_f64(),
            "failures":0,
            "retries":0,
            "cost":null,
        }))?
    );
    Ok(())
}
