# Stage 01 Implementation Progress v2

## Increment I2 — Atomic canonical mutation and durable outbox

**Status:** completed on 2026-07-11

### Delivered

- First-version lifecycle schema initialization and strict reopen validation.
- Process-wide storage write mutex covering read-modify-write, sequence allocation, and commit.
- Atomic RocksDB batches for node/edge create, node replacement, labels, node properties, edge/node deletion, and cascade deletion.
- Monotonic durable mutation sequence, typed entity state/tombstones, ordered outbox reads, and latest-sequence API.
- Cascade allocates one contiguous event per incident edge followed by the node event in one batch.
- Idempotent label writes publish no event; committed full node replacements preserve version/event behavior.
- Removed obsolete independent index/version put/delete helpers after mutation consolidation.

### Verification evidence

- Verification: lifecycle integration tests → ordered create/update/delete events, no-op labels, cascade ordering, reopen, and 8 concurrent writers all pass → **pass**.
- Verification: `cargo clippy -p zlf-storage --all-targets -- -D warnings -W clippy::too_many_lines` → no warnings → **pass**.
- Verification: `python3 scripts/check-rust-size.py` and `git diff --check` → no violations → **pass**.
- Verification: `cargo test --workspace` → complete deterministic workspace suite passes; only documented Ollama/wiki tests ignored → **pass**.
- Verification: cumulative I2 self-review → duplicate cascade-sequence and non-cascade node deletion findings fixed; no unresolved critical/major findings → **pass**.

### Remaining Stage 01 work

I3 adds explicit node/edge property patches, entity ambiguity handling, public WAM/JSON APIs, and edge identity lookup. I4 must route bulk graph loading through a rebuild marker. I5–I7 deliver profiles/chunks/coordinator/generations/status/wait.
