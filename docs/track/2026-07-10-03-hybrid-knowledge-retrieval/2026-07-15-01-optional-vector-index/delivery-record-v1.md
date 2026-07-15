# Delivery record v1

## Outcome

Accepted and delivered.

Vector embedding is opt-in and disabled by default. Enabled databases choose exact or HNSW. HNSW rebuild is asynchronous/immutable and exact automatically handles absent, stale, rebuilding, incompatible, or corrupt ANN state. Terminal configuration/status/rebuild contracts and batch-import guidance are documented.

The independent future Tree-sitter/code repository indexing track is recorded at `docs/track/2026-07-15-01-code-indexing/` with ingestion, graph, retrieval, and benchmark stages.

## Evidence

- Full workspace tests passed.
- Strict all-target Clippy passed.
- Formatting, Rust size, and diff checks passed.
- Focused disabled/exact/HNSW/reopen/corruption tests passed.

## Commit

Recorded by the delivery commit containing this file.
