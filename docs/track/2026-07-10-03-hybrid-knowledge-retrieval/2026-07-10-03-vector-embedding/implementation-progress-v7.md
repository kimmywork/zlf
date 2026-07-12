# Stage 03 Implementation Progress v7

## Increment V5b — Exact vector runtime cutover

**Status:** completed on 2026-07-11.

### Delivered

- Removed the node-only `VectorIndex`/`VectorEntry` implementation and its direct writer, in-memory queue, persistent prototype queue, blocking WAM adapter, CLI `index_embedding`, and CLI `similar` paths.
- `ZlfDatabase` now opens a generation-scoped `ExactVectorStore`, registers the immutable default model profile, bootstraps validated vector generation metadata, and catches canonical mutations through `VectorEmbeddingTarget`.
- Node/edge/property/Prolog mutations and profile changes now catch up both BM25 and vector lifecycle targets.
- Async facade methods process durable document batches and prepare query-text vectors with any `zlf_embed::EmbeddingProvider` outside WAM.
- WAM `vector_similar/3` now resolves all exact source chunks for the active generation/model, excludes the source entity, aggregates target scores deterministically, and never chooses an arbitrary node vector.
- New end-to-end facade test covers profile activation, canonical writes, durable fake embedding, exact publication, graph/WAM similarity join, source exclusion, and query embedding.

### Verification

- `cargo test -p zlf-query --test vector_database_facade`
- `cargo test -p zlf-prolog --test index_wam_provider --test storage_wam_provider`
- `cargo test -p zlf-index`
- `cargo test -p zlf-query`
- `cargo test -p zlf-cli --tests`
- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Next

Run cumulative Stage 03 review and fresh workspace acceptance. Confirm generation status/waits, local evidence, Ollama OpenAI-compatible evidence, and no remaining prototype vector symbols before marking Stage 03 done.
