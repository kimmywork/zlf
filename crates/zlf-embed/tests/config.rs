use zlf_embed::{EmbeddingConfig, ProviderType};

#[test]
fn config_serialization_round_trips() {
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
