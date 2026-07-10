---
status: proposed
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
- Define a pluggable ANN contract and benchmark at least one embedded candidate before selection.
- ANN persistence/rebuild/reopen must preserve or measurably reproduce Recall@k and deterministic tie rules.
- Support top-k, threshold, model/generation filters, source inclusion/exclusion, and metadata filters supported by the chosen backend.
- Support both source-document similarity and query-text/vector retrieval without performing remote embedding HTTP calls inside the WAM execution loop.
- Embedding jobs carry content fingerprint, source version, model, dimension, attempts, and timestamps; use provider batch APIs where available.
- Retry/backoff/dead-letter behavior and stale-job suppression are observable.
- API keys and source text are excluded from logs/reports by default.

## Verification

- Exact search is tested against an independent cosine oracle, including zero vectors, NaN/Inf rejection, dimensions, model isolation, updates, and deletes.
- ANN reports Recall@1/10/100 versus exact, p50/p95/p99, QPS, build/update throughput, peak RSS, and disk size at 10K/100K/1M/full tiers.
- Embedding generation reports provider/model, batch size, tokens or characters, throughput, failures/retries, and cost where available, separately from retrieval.
- Deterministic precomputed vectors make core CI independent of Ollama/network; local `bge-m3:latest` remains an opt-in end-to-end gate.

## Non-goals

- Training/fine-tuning models, mandatory GPU support, or a distributed vector service.
- Treating synthetic embedding recall as semantic relevance evidence.

## Open question

May the implementation add an embedded ANN dependency, or must it remain RocksDB/current-dependency only?
