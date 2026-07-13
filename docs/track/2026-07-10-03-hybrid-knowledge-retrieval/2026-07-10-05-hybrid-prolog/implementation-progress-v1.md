# Stage 05 Implementation Progress v1

## Increment H0 — Structured retrieval and fusion contracts

**Status:** completed on 2026-07-13.

### Delivered

- Structured lexical/vector/hybrid request contract with text, vector, source-document, and prepared-handle query forms.
- Explicit validated top-k, candidate, page, page-count, and answer budgets; invalid or effectively unbounded shapes fail with `RetrievalContractError`.
- Field/profile selectors, model/analyzer generation selectors, threshold, source exclusion, graph-filter goal, document/entity aggregation, explain flag, and event/validity filters.
- Retrieval hits preserve stable `IndexDocumentId`, source range, per-retriever raw score/rank/generation/watermark, fused score/rank, and optional explanation.
- Versioned RRF baseline `k=60`; raw BM25/vector scores are never added.
- RRF deduplicates each retriever by first rank, treats missing retrievers as no contribution, ranks by fused score, and breaks exact ties by canonical document identity.
- Oracle fixtures cover rank arithmetic, exact fused ties, duplicate input, missing retriever, top-k truncation, finite scores, and invalid request/fusion shapes.

### Verification

- `cargo test -p zlf-index --test retrieval_contracts`
- `cargo clippy -p zlf-index --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Next

H1 adds immutable request-scoped prepared retrieval handles, resolves generation/watermark/model state, and performs remote query embedding before WAM execution.
