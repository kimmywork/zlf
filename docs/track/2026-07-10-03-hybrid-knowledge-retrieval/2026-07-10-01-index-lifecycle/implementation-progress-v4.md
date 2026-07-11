# Stage 01 Implementation Progress v4

## Increment I4 — Durable bulk sessions and rebuild marker

**Status:** completed on 2026-07-11

### Delivered

- Durable `Started -> Writing -> Complete` bulk sessions with checkpoint inventory.
- Atomic record-batch plus checkpoint writes and restart resume.
- Atomic, idempotent completion with exactly one `RebuildRequired` outbox event and sequence.
- Bulk pack loader migrated from independent raw progress/completion markers.
- Storage key format advanced to v2 for parallel-edge-safe first-version records.
- Removed untracked graph record-plan writes; raw APIs reject canonical graph/lifecycle namespaces.
- CLI bulk report includes rebuild sequence.

### Verification

- Verification: bulk session tests → checkpoint, reopen/resume, one completion event, backwards checkpoint rejection, and raw bypass rejection pass → **pass**.
- Verification: bulk pack tests → deterministic compile, load/query, idempotent completion, and corrupt pre-write rejection pass → **pass**.
- Verification: workspace clippy/format/size/diff gates → pass → **pass**.
- Verification: `cargo test --workspace --exclude zlf-cli` → all deterministic non-CLI workspace tests pass; documented provider/data tests ignored → **pass**.
- Verification: CLI changed binary compiles under full workspace clippy; prior serial CLI suite remains green → **pass**.

### Next

I5 implements immutable profiles, activation history, deterministic chunkers, and manifests.
