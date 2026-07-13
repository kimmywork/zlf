# Stage 05 Implementation Progress v4

## Increment H3 — Ranked paging and bound lookup pushdown

**Status:** completed on 2026-07-13.

### Delivered

- Shared validated `IndexPageRequest`/`IndexPage<T>` contract with stable offsets, finite page/candidate limits, candidates scanned, next offset, and conservative candidate-budget exhaustion.
- BM25 ranked pages preserve score/document ordering and push bound entity IDs into Tantivy Boolean term filters before ranking.
- Exact-vector pages preserve score/document ordering; vector queries now support include/exclude graph entities in addition to full document IDs.
- `vector_similar/3` pushes a bound target node into exact-vector filtering for every source chunk rather than ranking a global candidate set and unifying afterward.
- Event and validity stores maintain additional generation-scoped graph-entity indexes atomically with time/start/end/open/document indexes.
- Event range and validity containment/overlap support graph-entity-bound seeks, including page APIs for global event/validity queries and entity-bound event ranges.
- Temporal WAM predicates detect a bound result entity and use the graph-entity index before applying event/interval semantics.
- Generation remains fixed by each provider's active generation snapshot; no cross-generation page can be mixed.
- Refactored BM25 writer, paging, temporal entity, temporal provider, and vector filter helpers to preserve the repository source-size policy.

### Verification

- `cargo test -p zlf-index --test retrieval_contracts --test bm25_backend --test vector_exact --test temporal_event_store --test temporal_validity_store`
- `cargo test -p zlf-prolog --test index_wam_provider --test bounded_index_provider`
- `cargo test -p zlf-query --test retrieval_preparation --test temporal_lifecycle --test vector_database_facade`
- `cargo clippy -p zlf-index -p zlf-prolog --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Next

H4 implements the prepared `retrieve/4` facade, lexical/vector/hybrid plans, RRF, graph/rule/temporal filtering, progressive retrieval-first execution, and exactness/exhaustion planner metadata.
