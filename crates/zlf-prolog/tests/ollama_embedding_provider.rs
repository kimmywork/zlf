use zlf_prolog::wam::{BlockingEmbeddingProvider, Embedder};

#[test]
#[ignore = "requires local Ollama with bge-m3:latest pulled"]
fn ollama_bge_m3_embedding_provider_returns_vector() {
    let endpoint =
        std::env::var("OLLAMA_ENDPOINT").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let embedder = BlockingEmbeddingProvider::ollama_bge_m3(endpoint).unwrap();

    let embedding = embedder.embed("软件工程师").unwrap();

    assert!(!embedding.is_empty());
    assert_eq!(embedder.model(), "bge-m3:latest");
}
