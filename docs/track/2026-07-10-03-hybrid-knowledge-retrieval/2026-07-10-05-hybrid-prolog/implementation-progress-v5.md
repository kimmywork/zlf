# Stage 05 Implementation Progress v5

## Increment H4 — Prepared `retrieve/4`, fusion, and filters

**Status:** completed on 2026-07-13.

### Delivered

- Synchronous execution of immutable prepared retrieval handles over lexical, vector, or hybrid modes; no embedding provider is reachable from execution.
- BM25 and exact-vector ranked pages are collected only up to the validated candidate/page budget and converted to document-scoped retriever ranks with pinned generation/watermark provenance.
- Hybrid execution uses the frozen RRF `k=60` contract; raw BM25 and vector scores remain diagnostics and are never directly added.
- Optional entity aggregation uses a separate entity identity before fusion; document aggregation remains the default.
- Exact source-document exclusion applies to both lexical and vector candidates; field constraints are pushed into both backends.
- Event-range, valid-at, and valid-overlap filters use generation-scoped graph-entity temporal indexes.
- Graph/label/property/rule filter goals bind `Entity` and run through the existing WAM/provider/rule path for each bounded fused candidate.
- Bound `retrieve/4` entity arguments push entity filters into Tantivy and exact-vector search before ranking.
- `retrieve(PreparedHandle, Options, Entity, Hit)` is registered as an index predicate and returns stable entity/field/chunk identity, per-retriever rank/score/generation/watermark, fused rank/score, strategy, candidate exhaustion, and exact-filtered-top-k metadata.
- Planner explain reports `HybridRetrieval` rather than a generic external index path.
- Execution metadata records mode, bound-entity or retrieval-first strategy, lexical/vector pages and candidates, fused candidates, graph/temporal rejection counts, conservative candidate-budget exhaustion, and exactness.

### Verification

- End-to-end profile/outbox BM25+embedding+temporal fixture covers lexical-only, vector-only, hybrid RRF, one-item pages, exact source exclusion, temporal filtering, ordinary label filtering, bound-entity pushdown, `retrieve/4`, and planner output.
- `cargo test -p zlf-query --test retrieval_execution --test query_plan --test retrieval_preparation`
- `cargo test -p zlf-index --test vector_contracts --test vector_exact`
- `cargo clippy -p zlf-index -p zlf-prolog -p zlf-query --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Next

H5 adds retrieval-specific compact proof leaves, generation/watermark table dependencies and invalidation, explicit non-tableable live combinations, and minimum-watermark waits before WAM execution.
