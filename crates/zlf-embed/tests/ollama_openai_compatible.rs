use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

use zlf_embed::{EmbeddingConfig, EmbeddingProvider, OllamaProvider, ProviderType};

#[tokio::test]
async fn ollama_uses_openai_compatible_batch_endpoint_and_ordering() {
    let (endpoint, received, server) = mock_server();
    let provider = OllamaProvider::new(EmbeddingConfig {
        provider: ProviderType::Ollama,
        api_endpoint: endpoint,
        api_key: None,
        model: "bge-m3:latest".into(),
        dimension: 2,
    });
    let vectors = provider.embed_batch(&["first", "second"]).await.unwrap();
    assert_eq!(vectors, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    assert_eq!(provider.name(), "ollama_openai_compatible");
    let request = received.recv().unwrap();
    assert!(request.starts_with("POST /v1/embeddings HTTP/1.1"));
    let body = request.split("\r\n\r\n").nth(1).unwrap();
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(json["model"], "bge-m3:latest");
    assert_eq!(json["input"], serde_json::json!(["first", "second"]));
    server.join().unwrap();
}

fn mock_server() -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_request(&mut stream);
        sender.send(request).unwrap();
        let body = serde_json::json!({
            "data": [
                {"index": 1, "embedding": [0.0, 1.0]},
                {"index": 0, "embedding": [1.0, 0.0]}
            ],
            "model": "bge-m3:latest"
        })
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    (format!("http://{address}"), receiver, server)
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = stream.read(&mut buffer).unwrap();
        bytes.extend_from_slice(&buffer[..count]);
        let request = String::from_utf8_lossy(&bytes);
        if let Some(header_end) = request.find("\r\n\r\n") {
            let content_length = request[..header_end]
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .or_else(|| {
                    request[..header_end]
                        .lines()
                        .find_map(|line| line.strip_prefix("Content-Length: "))
                })
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or_default();
            if bytes.len() >= header_end + 4 + content_length {
                return String::from_utf8(bytes).unwrap();
            }
        }
    }
}
