# Implementation progress v1

## Delivered

- Embedding/vector capability defaults to disabled through `ZlfDatabaseOptions` and `zlf-config`.
- Exact and HNSW are explicit strategies; HNSW always retains exact as authority/fallback.
- Disabled profiles, embedding operations, vector predicates, and vector/hybrid preparation return typed errors.
- Durable HNSW publications bind canonical records/model/options, atomically publish, reopen, and reject corruption/stale identity.
- HNSW rebuilds run on a coalescing background worker; stale/missing/corrupt ANN routes to exact.
- CLI supports `vector_index_status` and `rebuild_vector_index`; README recommends one rebuild per completed import batch.
- Disabling vector removes its coordinator registration/jobs so outbox compaction is not blocked.
- Independent Tree-sitter/code-index track recorded at `docs/track/2026-07-15-01-code-indexing/`.

## Verification

- `cargo test --workspace` passed.
- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines` passed.
- `cargo fmt --all`, Rust size policy, and `git diff --check` passed.
