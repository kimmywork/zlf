# Stage 04 Implementation Progress v3

## Increment T2 — Ordered validity interval indexes

**Status:** completed on 2026-07-11.

### Delivered

- Generation-scoped `valid/by-start`, `valid/by-end`, `valid/open-end`, and `valid/by-entity` binary key families.
- Atomic multi-index put/delete and explicit schema rejection.
- Correct half-open `valid_at` containment and `valid_overlaps` queries, including adjacent endpoints and positive-infinity open ends.
- Write-side refreshed generation statistics (counts and endpoint bounds) estimate start-side versus end-side candidates without query-time full-database scans.
- Auto-selected start/end range seeks; finite end candidates and open ends are merged explicitly.
- Candidate scans stream records and retain only a bounded top-limit heap ordered by validity start/record ID.
- Entity lookup, generation isolation, update/supersede/delete, fresh reopen, zero-limit and empty-range validation.

### Verification

- `cargo test -p zlf-index --test temporal_validity_store --test temporal_contracts`
- `cargo clippy -p zlf-index --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`

Differential fixtures compare store results with independent containment/overlap oracles and cover finite/open records, endpoint path selection, candidate counts, adjacent intervals, i64 maximum, updates, deletes, isolation, limits, and reopen.

### Next

T3 replaces the creation-date prototype runtime with profile-declared event/validity lifecycle projection, generation-scoped stores, approved WAM predicates, and provenance/planner access paths.
