# Stage 03 Implementation Progress v4

## Increment V2b — Batch embedding worker and lifecycle publication

**Status:** completed on 2026-07-11.

### Delivered

- Pluggable query/document batch provider boundary outside WAM execution.
- Durable worker claims bounded batches, loads source text from generation-scoped manifests, applies separate query/document templates, normalizes configured outputs, and rejects dimensions, non-finite values, zero vectors, and batch cardinality mismatches.
- Provider retry/permanent classification, delayed retries, dead letters, stale source-version suppression, lease recovery, and redacted diagnostics.
- Exact-store publication precedes durable completion, making crash replay idempotent.
- `VectorEmbeddingTarget` consumes canonical mutation outbox events, projects active profile chunks, removes stale vectors before updates, enqueues fingerprinted jobs, handles deletes/profile versions/rebuild, and uses target-scoped manifests.
- Immutable durable `EmbeddingModelProfileStore` registry.
- Deterministic fake providers cover transforms, batching, normalization, retries, dead letters, stale jobs, updates, deletes, replay, and source-text exclusion.

### Verification

- `cargo test -p zlf-query --test embedding_jobs --test embedding_worker_v2 --test vector_lifecycle --test model_profiles`
- `cargo test -p zlf-index --test vector_exact --test vector_contracts`
- `cargo clippy -p zlf-index -p zlf-query --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Next

V3 evaluates optional `hnsw_rs` persistence behind `AnnBackend`. Exact search remains the functional production backend and correctness fallback if ANN integration is not straightforward.
