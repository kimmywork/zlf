# Change Note v5: Vector embedding is opt-in and HNSW is a strategy

**Date:** 2026-07-15  
**Status:** accepted by user direction

## Decision

Vector embedding/indexing is no longer an unconditional database subsystem. It is disabled by default and must be explicitly enabled. Enabled deployments select `exact` or `hnsw`; HNSW is an immutable derivative strategy while exact RocksDB remains source of truth and fallback.

## Behavior

- Disabled vector profiles, embedding operations, vector predicates, and vector/hybrid requests return an explicit typed error.
- HNSW rebuild occurs asynchronously and does not block reads.
- Missing, incomplete, incompatible, or corrupt ANN publications fall back to exact search.
- Operators should batch knowledge imports and request one rebuild per completed batch.
- No existing database compatibility or migration contract is added before first release.

## Motivation

Embedding cost and relevance vary by workload. Code repositories contain many exact symbols and strong structural relationships, where BM25 and graph traversal are usually a better baseline. Optional embedding avoids remote inference, vector storage, queue, and ANN rebuild costs for those deployments.

## Consequences

Existing tests and examples that require vectors must explicitly enable exact or HNSW strategy. HNSW does not replace exact. Tree-sitter/code indexing is tracked independently under `docs/track/2026-07-15-01-code-indexing/`.
