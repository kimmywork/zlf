# Stage 04 Implementation Progress v2

## Increment T1 — Ordered event-time RocksDB index

**Status:** completed on 2026-07-11.

### Delivered

- Generation-scoped `event/by-time` and `event/by-entity` binary key families.
- Atomic dual-index put/delete and explicit incompatible-schema rejection.
- Bounded half-open range and UTC-day seeks using ordered signed-microsecond keys.
- Separate strict `before(< instant)` and `after(> instant)` APIs with positive limits.
- Document-scoped lookup without scanning unrelated event records.
- Duplicate instants preserved by stable record ID; chronological/record-ID ordering is deterministic.
- Candidate counts expose actual bounded records scanned.
- Update/supersede via idempotent old-record delete plus new-record put; generation isolation and reopen are covered.

### Verification

- `cargo test -p zlf-index --test temporal_event_store --test temporal_contracts`
- `cargo clippy -p zlf-index --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`

Boundary tests cover start inclusion/end exclusion, duplicate timestamps, day boundaries, before/after strictness, zero limits, generation isolation, entity lookup, update/delete, i64 maximum, and fresh reopen.

### Next

T2 implements validity by-start/by-end/open-end/by-entity indexes with bounded endpoint selection, containment/overlap semantics, candidate counts, updates/deletes, and reopen.
