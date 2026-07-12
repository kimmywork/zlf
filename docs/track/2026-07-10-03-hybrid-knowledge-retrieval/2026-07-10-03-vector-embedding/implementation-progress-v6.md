# Stage 03 Implementation Progress v6

## Increment V5a — Exact scale and Ollama OpenAI-compatible evidence

**Status:** completed on 2026-07-11.

### Delivered

- Reproducible 1K/10K exact-vector release benchmark with deterministic 64-dimensional vectors, atomic build/update, warm/fresh-reader p50/p95/p99, RSS, disk, MRR, and Recall@10.
- Frozen generous regression budgets in `research/vector-exact-local-2026-07-11.json`.
- Ollama provider now uses the OpenAI-compatible `/v1/embeddings` endpoint and true input-array batch requests rather than sequential legacy `/api/embeddings` calls.
- Shared strict OpenAI-compatible response parsing, `data[].index` ordering, cardinality checks, status checks, and source-safe error classes.
- `DurableEmbeddingWorker` is async and directly accepts any `zlf_embed::EmbeddingProvider`; remote HTTP remains outside the WAM execution loop.
- Deterministic mock HTTP protocol test plus successful opt-in local Ollama `bge-m3:latest` smoke gate at 1024 dimensions.

### Evidence

- 10K exact vectors: 8.74 ms build, 5.86 ms warm p99, 6.19 ms fresh-reader p99, 59.9 MB RSS, 3.90 MB disk, MRR/Recall@10 1.0.
- Local Ollama OpenAI-compatible Chinese query: pass in 3.11 seconds; 1024 finite, non-zero components.
- See `research/vector-exact-local-2026-07-11.{json,md}` and `research/ollama-openai-compatible-2026-07-11.md`.

### Next

Remove the active node-only prototype vector API/provider/writer path, expose exact generation/model/document retrieval through the query/WAM facade, then perform V6 cumulative acceptance.
