# Delivery Record: Stage 04 Temporal Semantics and Indexes v1

## Outcome

**Accepted and delivered on 2026-07-13.**

Stage 04 replaces implicit creation-date scanning with explicit event-time and valid-time records, ordered generation-scoped RocksDB indexes, durable lifecycle projection, WAM predicates, planner provenance, independent differential oracles, and local scale evidence.

## Delivered capabilities

- UTC signed-microsecond contracts and lexicographically ordered codec.
- Event day/range/before/after/document seeks with duplicate preservation.
- Half-open finite/open validity containment and overlap queries using start/end/open/document key families.
- Deterministic result ordering, limits, candidates scanned, generation/watermark/access-path provenance.
- Atomic single and batch put/delete/replace, generation isolation, reopen, and persisted endpoint statistics.
- Profile-declared event/valid-from/valid-to scalar and array extraction.
- Durable target manifests and canonical outbox convergence for update, delete, replay, rebuild, and profile-version retirement.
- Generation-managed database facade and removal of the legacy creation-date temporal prototype.
- WAM predicates `temporal_on/2`, `temporal_between/3`, `valid_at/2`, and `valid_overlaps/3`.
- Planner explain paths `TemporalEventRange` and `ValidityInterval`.
- Reproducible 1K/10K/100K uniform, skewed, and long-open release benchmark.

## Acceptance evidence

Passed on the delivery tree:

```bash
cargo fmt --all -- --check
python3 scripts/check-rust-size.py
git diff --check
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
```

Focused temporal contract, store, lifecycle, provider, and planner tests also pass. Machine-readable benchmark reports and methodology are under `research/temporal-local-*2026-07-11*`.

At 100K records per kind, build of 200K records took 377.9–434.2 ms; worst query p99 was 61.93 ms; peak process RSS was 531.8 MB; and each distribution used less than 97 MB disk. Every measured query was asserted against an independent oracle.

## Deferred by scope

- Full bitemporal algebra and transaction-time query language.
- Interval trees/bucket derivatives without evidence of a frozen-budget violation.
- Combined graph/rule/BM25/vector/temporal fusion and top-k/cut/tabling acceptance, owned by Stage 05.
- Combined enterprise/BEIR/multilingual stress orchestration, owned by Stage 06.

## Commits

- `6b95aed` temporal contracts and oracles.
- `a011316` ordered event-time store.
- `623e69b` ordered validity store.
- `590175a` temporal lifecycle/runtime cutover.
- `47ce48b` temporal local scale baseline and atomic batches.
