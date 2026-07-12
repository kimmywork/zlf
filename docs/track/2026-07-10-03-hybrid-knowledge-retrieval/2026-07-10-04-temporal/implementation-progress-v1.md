# Stage 04 Implementation Progress v1

## Increment T0 — Temporal semantics, codec, and oracle

**Status:** completed on 2026-07-11.

### Delivered

- Distinct versioned event-instant and validity `[from,to)` record contracts with generation, canonical document identity, source version, and stable record ID.
- UTC RFC3339 parser requiring explicit offsets; ISO dates map to UTC midnight.
- UTC date-day conversion to `[00:00:00Z,next-day 00:00:00Z)` including leap days.
- Checked non-empty half-open query ranges; empty/reversed validity records fail closed; `None` end is positive infinity.
- Sign-bit-flipped big-endian signed-microsecond codec preserving full chronological byte order.
- Provenance contract with generation, watermark, access path, and candidate count.
- Independent event-range, validity-containment, and overlap filter oracles with deterministic ordering.

### Verification

- `cargo test -p zlf-index --test temporal_contracts`
- `cargo clippy -p zlf-index --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`

Fixtures cover negative/positive epoch extremes, explicit DST offsets, leap day, ambiguous timezone rejection, duplicate instants, equal/adjacent endpoints, open ends, and invalid empty ranges.

### Next

T1 implements generation-scoped `event/by-time` and `event/by-entity` RocksDB keys with bounded day/range/before/after seeks and duplicate timestamp preservation.
