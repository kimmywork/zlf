use zlf_prolog::wam::{BlockingEmbeddingProvider, Embedder};

#[test]
#[ignore = "requires local Ollama with bge-m3:latest pulled"]
fn ollama_openai_compatible_bge_m3_returns_valid_vector() {
    let endpoint =
        std::env::var("OLLAMA_ENDPOINT").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let embedder = BlockingEmbeddingProvider::ollama_bge_m3(endpoint).unwrap();

    let embedding = embedder.embed("软件工程师").unwrap();

    assert_eq!(embedding.len(), 1024);
    assert!(embedding.iter().all(|value| value.is_finite()));
    assert!(embedding.iter().any(|value| *value != 0.0));
    assert_eq!(embedder.model(), "bge-m3:latest");
}
