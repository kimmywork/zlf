# Stage 05 Implementation Progress v2

## Increment H1 — Prepared async retrieval

**Status:** completed on 2026-07-13.

### Delivered

- Immutable process-local `PreparedRetrieval` entries addressed by typed request-scoped handles, with explicit lookup and release.
- Preparation validates the H0 request, resolves active lexical/vector/temporal generations, and snapshots each target's published watermark plus model identity/version.
- Requested analyzer/model generations fail before execution when they do not match the active generations.
- Hybrid/vector literal text is transformed, remotely embedded, normalized, and model-validated asynchronously before registry publication.
- Explicit vectors avoid remote calls and are validated for active metric, dimension, finite values, cosine nonzero, and normalization policy.
- Lexical and source-document query forms require no query embedding.
- Preparation, generation mismatch, embedding failure, invalid vector, snapshot failure, and unknown handle have typed errors.
- The WAM/provider execution path has no embedding-provider reference. Tests prepare once, perform registry lookups and a WAM graph query, and verify the provider call count remains unchanged.
- Shared vector-query validation now enforces dimension, finite, cosine-zero, and normalization requirements consistently in preparation and exact search.

### Verification

- `cargo test -p zlf-index --test vector_contracts --test vector_exact`
- `cargo test -p zlf-query --test retrieval_preparation`
- `cargo clippy -p zlf-index -p zlf-query --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Next

H2 enforces bounded provider materialization and verifies backtracking, exhaustion, cut/once, proof leaves, and peak answer limits before introducing any WAM-owned cursor.
