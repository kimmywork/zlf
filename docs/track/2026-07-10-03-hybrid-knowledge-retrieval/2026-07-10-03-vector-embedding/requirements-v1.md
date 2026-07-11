---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-10
version: 1
---

# Stage 03 Requirements: Vector and Embedding Retrieval

## Goal

Deliver model-safe multi-vector storage, a reliable embedding pipeline, and scalable nearest-neighbor retrieval with exact quality verification.

## Requirements

- Key vectors by indexed-document identity plus embedding model/version; support multiple fields/chunks/models per node.
- Validate dimension, finite values, metric, model identity, and normalization policy at ingestion.
- Retain exact cosine/dot-product search as a deterministic oracle and small-corpus backend.
- Define a pluggable ANN contract, but ship exact RocksDB retrieval first. Use `hnsw_rs` as the initial embedded ANN derivative if integration is straightforward; ANN does not block functional delivery and exact remains the oracle/fallback.
- ANN persistence/rebuild/reopen must preserve or measurably reproduce Recall@k and deterministic tie rules.
- Support top-k, threshold, model/generation filters, source inclusion/exclusion, and metadata filters supported by the chosen backend.
- Support both source-document similarity and query-text/vector retrieval without performing remote embedding HTTP calls inside the WAM execution loop.
- Define a versioned `EmbeddingModelProfile` with provider, model ID/revision, dimension, metric, normalization, maximum input, query/document templates, batch limits, and dense/sparse/multi-vector capabilities.
- Embedding jobs carry content fingerprint, source version, model profile, dimension, attempts, and timestamps; use provider batch APIs where available.
- Retry/backoff/dead-letter behavior and stale-job suppression are observable.
- API keys and source text are excluded from logs/reports by default.

## Verification

- Exact search is tested against an independent cosine oracle, including zero vectors, NaN/Inf rejection, dimensions, model isolation, updates, and deletes.
- ANN reports Recall@1/10/100 versus exact, p50/p95/p99, QPS, build/update throughput, peak RSS, and disk size at 1K–10K smoke and at most 100K chunks on the current M2 Pro/32-GiB machine.
- Embedding generation reports provider/model, batch size, tokens or characters, throughput, failures/retries, and cost where available, separately from retrieval.
- Deterministic precomputed vectors make core CI independent of Ollama/network; local `bge-m3:latest` remains an opt-in end-to-end gate.

## Non-goals

- Training/fine-tuning models, mandatory GPU support, or a distributed vector service.
- Treating synthetic embedding recall as semantic relevance evidence.

## Confirmed ANN policy

Embedded ANN crates are allowed. `hnsw_rs` is the initial optional choice after the exact path works; ANN format/version lifecycle must satisfy Stage 01 rebuild and generation contracts.

## Confirmed model strategy

Use a pluggable, versioned embedding model registry. Ollama `bge-m3:latest`, 1024-dimensional dense embedding is the default and first benchmark baseline. It is not hard-coded into physical vector identity, and deployments may approve a restricted profile set. Query and document encoding templates are separate profile fields. Sparse and multi-vector capabilities remain disabled until focused benchmarks justify them.
