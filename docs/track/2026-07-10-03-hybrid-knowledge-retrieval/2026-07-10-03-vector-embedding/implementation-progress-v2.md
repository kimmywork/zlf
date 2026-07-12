# Stage 03 Implementation Progress v2

## Increment V1a — Canonical exact RocksDB vector backend

**Status:** exact backend completed on 2026-07-11; embedding-job lifecycle publication continues in V2.

### Delivered

- Versioned RocksDB keys scoped by generation, model profile/version, and canonical indexed-document identity.
- Strict model/profile validation before atomic batch upsert; batch deletes are idempotent.
- Explicit incompatible-schema rejection and fresh-process reopen.
- Streaming exact cosine and dot-product search with f64 accumulation and bounded top-k heap memory.
- Threshold, source include/exclude, metadata, model, and generation filtering.
- Deterministic score-descending/document-ID-ascending tie order.
- Independent cosine fixture plus dimension, NaN, zero-vector, normalization, model/generation isolation, update, delete, filter, count, and reopen coverage.

### Verification

- `cargo test -p zlf-index --test vector_exact --test vector_contracts`
- `cargo clippy -p zlf-index --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`

### Next

V2 connects transformed document batches and durable embedding jobs to this canonical store, including lease/retry/dead/stale/fingerprint behavior and fake-provider crash tests. That increment also replaces the active node-only vector write path.
