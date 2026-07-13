# Stage 05 Implementation Progress v3

## Increment H2 — Bounded provider materialization

**Status:** completed on 2026-07-13.

### Delivered

- `IndexAnswerLimits` enforces a positive answer limit and a candidate limit that covers it; invalid shapes fail before provider execution.
- BM25, exact-vector, event, and validity provider paths pass the explicit candidate limit into their backends and truncate to the explicit answer limit before answers enter WAM.
- Exact-vector source-chunk aggregation is pruned to the candidate budget after each chunk, bounding the entity score map while preserving deterministic score/entity ordering.
- Shared provider metrics report calls, produced candidates/answers, peak materialized answers, candidate-budget saturation/exhaustion, and answer-budget truncation.
- Existing predicates retain a finite default of 10,000 candidates/answers until H3/H4 attach request-specific paging and budgets.
- Added provider-only proof execution so external index facts can produce normal compact fact proof leaves without requiring a storage handle.
- WAM verification covers deterministic backtracking, bounded peak answers, conservative exhaustion reporting, `once/1`, cut in a rule consumer, external proof leaves, and invalid limit rejection.
- No WAM-owned cursor was introduced; the bounded materialized path satisfies this increment's functional acceptance criteria.

### Verification

- `cargo test -p zlf-prolog --test bounded_index_provider --test index_wam_provider`
- `cargo clippy -p zlf-prolog --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Next

H3 adds backend paging/cursors and bound-document/entity lookup so selective bindings and request candidate/page budgets are pushed below provider materialization.
