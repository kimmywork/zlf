# Ollama OpenAI-compatible smoke verification — 2026-07-11

## Deterministic protocol verification

`zlf-embed/tests/ollama_openai_compatible.rs` runs without network dependencies against a local mock HTTP server and verifies:

- `POST /v1/embeddings` rather than the legacy `/api/embeddings` request;
- OpenAI-compatible `{model,input:[...]}` batching;
- response ordering by `data[].index`;
- provider identity `ollama_openai_compatible`;
- no authorization header requirement for local Ollama.

## Local Ollama gate

The local server and model were available, so the opt-in gate was executed:

```bash
curl -fsS --max-time 3 http://localhost:11434/api/tags
cargo test -p zlf-prolog --test ollama_embedding_provider -- --ignored --nocapture
```

Result: **pass** in 3.11 seconds for one Chinese query (`软件工程师`) using `bge-m3:latest`. The response contained exactly 1024 finite components and at least one non-zero component. No vector values, source content beyond the fixed test phrase, endpoint credentials, or API keys are recorded.

This is a connectivity/dimension smoke gate, not an embedding quality or sustained-throughput benchmark.
