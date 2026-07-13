# Stage 05 Implementation Progress v6

## Increment H5 — Proof, table dependencies, and freshness

**Status:** completed on 2026-07-13.

### Delivered

- Retrieval requests now carry validated per-target minimum published watermarks and a bounded wait timeout; only `bm25`, `vector`, and `temporal` targets are accepted.
- Async preparation waits before snapshotting generations or embedding and returns typed `WatermarkTimeout { target, minimum, published }` rather than entering WAM with insufficient freshness.
- `retrieve/4` external answers generate stable `ProofKind::Index` leaves with `index:retrieve:*` IDs. Their compact substitutions contain handle, entity/document identity, ranks, scores, generations, watermarks, and exactness metadata, never source text.
- Prepared `retrieve/4` calls can be tabled when the immutable handle and options are bound. The table variant key therefore includes the prepared handle/options, whose registry entry pins generation and watermark dependencies.
- Unbound/live `retrieve/4` table calls fail explicitly as non-tableable instead of being cached under an unsafe latest-state key.
- External prepared retrieval tables use ordinary provider answers without requiring synthetic user rules.
- Retrieval tables depend on the `retrieve/4` predicate and are selectively invalidated after canonical index catch-up, successful embedding publication, and generation activation/rollback.
- Repeated identical prepared calls hit the table cache; subsequent indexed graph mutation marks the table stale.
- Proof execution now includes the prepared retrieval provider, keeping normal and proof query behavior aligned.

### Verification

- `cargo test -p zlf-index --test retrieval_contracts`
- `cargo test -p zlf-query --test retrieval_execution --test retrieval_preparation --test tabling_integration`
- `cargo test -p zlf-prolog --test proof_terms --test tabling`
- `cargo clippy -p zlf-index -p zlf-prolog -p zlf-query --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Next

H6 runs identical-judgment lexical/vector/hybrid quality comparisons and local ACL/filter/top-k scale tiers, recording candidates, selectivity, p50/p95/p99, peak materialized answers, RSS, and measured quality without assuming fusion improvement.
