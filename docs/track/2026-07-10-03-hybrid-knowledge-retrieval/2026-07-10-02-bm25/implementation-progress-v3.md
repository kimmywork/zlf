# Stage 02 Implementation Progress v3

## Increment B2/B3a — Field/chunk retrieval and lifecycle target

**Status:** functional slice completed on 2026-07-11; B3 remains open for generation directory publication.

### Delivered

- Tantivy documents now persist canonical node/edge, field, and chunk identity instead of one vector-like node key.
- Field filters, bounded candidate over-fetch, field weights, stable document tie-breaking, and optional Tantivy score explanations.
- Explicit backend schema validation and first-version field layout.
- Target-scoped durable manifests avoid cross-backend reconciliation conflicts.
- Durable `Bm25IndexTarget` consumes Stage 01 outbox jobs, active profiles, chunks, updates, deletes, bulk/profile rebuild events, stale suppression, and retries.
- Manifest publication follows backend mutation so a failed partial apply is safely replayed.
- `ZlfDatabase` node/edge/property/Prolog/profile paths synchronously catch up the BM25 target instead of directly maintaining a second token-count path.
- Active profile field weights are used by the structured `search_bm25` facade.
- Profile-version changes remove documents belonging to the superseded version.

### Verification evidence

- `cargo test -p zlf-index --test bm25_backend`
- `cargo test -p zlf-query --test bm25_lifecycle`
- `cargo test -p zlf-prolog --test index_wam_provider --test storage_wam_provider`
- `cargo clippy -p zlf-index -p zlf-query --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Remaining before Stage 02 acceptance

- Place physical Tantivy indexes under generation-scoped directories and connect validation/activation/rollback to `GenerationManager`.
- Add rebuild/reopen/crash and independent corpus-oracle acceptance coverage at the lifecycle boundary.
- Expose the final structured lexical request/hit/explanation contract through the Prolog/provider surface in Stage 05.
