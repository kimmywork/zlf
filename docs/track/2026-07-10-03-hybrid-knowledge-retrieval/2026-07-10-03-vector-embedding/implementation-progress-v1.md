# Stage 03 Implementation Progress v1

## Increment V0 — Model-safe vector contracts

**Status:** completed on 2026-07-11.

### Delivered

- Canonical `VectorKey` identity includes generation, model profile/version, and full indexed-document entity/field/chunk identity.
- Versioned `VectorRecord`, `VectorQuery`, `VectorHit`, and durable `EmbeddingJob` envelopes.
- Strict profile compatibility checks for model revision, dimension, metric, finite values, normalization, and cosine zero vectors.
- Explicit include/exclude source and metadata query filters.
- Versioned query/document templates with deterministic maximum-input enforcement.
- `bge_m3_dense_v1` remains an ordinary validated registry artifact.

### Verification

- `cargo test -p zlf-index --test vector_contracts`
- `cargo clippy -p zlf-index --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`

### Next

V1 replaces the node-only prototype `VectorIndex` with a generation/model/document keyed exact RocksDB store and independent f64 oracle tests.
