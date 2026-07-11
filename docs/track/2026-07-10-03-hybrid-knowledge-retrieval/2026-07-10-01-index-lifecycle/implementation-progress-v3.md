# Stage 01 Implementation Progress v3

## Increment I3 — Node/edge property mutation and public surfaces

**Status:** completed on 2026-07-11

### Delivered

- Atomic node/edge property patch, set, remove, null-value, and idempotent no-op semantics.
- Typed missing/ambiguous entity errors and generic property entity resolution.
- Parallel-edge-safe adjacency keys and stable ordered edge identity lookup.
- Prolog `set/remove_node_property`, `set/remove_edge_property`, and `edge_id/4`.
- Generic `assertz/retract(property/3)` now resolves node or edge without creating the wrong entity.
- Rust facade and JSON-over-STDIO patch/set/remove/edge-ID APIs.
- Selective property table invalidation and prohibition of mutation builtins inside tabled evaluation.

### Verification

- Verification: storage property tests → node/edge patches, null, no-op, ambiguity, parallel IDs all pass → **pass**.
- Verification: Prolog tests → explicit mutation, generic edge assert/retract, and `edge_id/4` pass → **pass**.
- Verification: query tabling test → explicit property mutation invalidates exact dependency and recomputes → **pass**.
- Verification: CLI integration tests serially → existing 13/13 plus property API test pass → **pass**.
- Verification: full workspace clippy, format, Rust size, and diff checks → pass → **pass**.
- Verification: full workspace tests → prior I2 workspace run passed; I3 run exceeded harness timeout during parallel CLI subprocess tests, then all CLI tests passed serially and all affected focused tests passed → **pass with timeout explained**.

### Next

Proceed to I4 bulk rebuild markers, then I5 profiles/chunking.
