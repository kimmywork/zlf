# Stage 04 Implementation Progress v4

## Increment T3 — Temporal lifecycle, WAM, and planner cutover

**Status:** completed on 2026-07-11.

### Delivered

- `TemporalIndexTarget` consumes canonical outbox events and only projects profile-declared `Event`, `ValidFrom`, and `ValidTo` fields; arbitrary strings and node creation timestamps are no longer auto-indexed.
- Scalar or array event fields preserve multiple stable temporal records; one validated from/to array pair supports multiple finite intervals, while missing `ValidTo` creates open intervals.
- Durable entity/profile/version temporal manifests reconcile update, supersede, delete, configuration rebuild, stale replay, and profile-version retirement.
- `ZlfDatabase` opens generation-scoped event/validity stores, bootstraps validated temporal generation metadata, and catches temporal lifecycle alongside BM25/vector.
- Removed the creation-date scan prototype `TemporalIndex`/`TemporalEntry` path.
- WAM predicates now implement approved semantics: `temporal_on/2`, half-open `temporal_between/3`, `valid_at/2`, and `valid_overlaps/3`.
- Planner distinguishes `TemporalEventRange` and `ValidityInterval` pushed-down access paths.
- End-to-end facade tests cover profile activation, canonical mutation, event and finite/open validity queries, boundary exclusion, arrays, update/delete/replay, and graph-visible entity identity.

### Verification

- `cargo test -p zlf-query --test temporal_lifecycle --test query_plan`
- `cargo test -p zlf-prolog --test index_wam_provider`
- `cargo clippy -p zlf-index -p zlf-prolog -p zlf-query --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`
- `git diff --check`

### Next

T4 runs differential local scale tiers over uniform events, skewed histories, finite and long-lived open intervals. It records build/update throughput, point/overlap candidates, p50/p95/p99, RSS, and disk before T5 cumulative acceptance.
