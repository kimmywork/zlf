# Stage 01 Implementation Progress v1

## Increment I1 — Shared contracts

**Status:** completed on 2026-07-11

### Delivered

- `zlf-core`: typed `EntityRef` and validated atomic `PropertyPatch`.
- `zlf-storage`: schema-versioned mutation sequence, entity state, mutation event/kind, and receipt contracts.
- `zlf-index`: split modules for indexed-document identity and canonical keys, profiles/chunking, embedding model registry, generations/status, retrieval request/hit, and metrics.
- Data-defined `bge_m3_dense_v1` default without coupling physical vector identity to 1024 dimensions.
- Contract tests for typed identity, separator-safe keys, patch conflict/null semantics, bincode enum round trips, mutation source versions, and model defaults.

### Implementation correction

The first contract draft used internally tagged serde enums. Focused tests showed `bincode` cannot deserialize that representation (`DeserializeAnyNotSupported`). Persisted enums were changed to bincode-compatible externally tagged serde representation. This preserves both binary persistence and typed JSON representation and required no design/scope change.

### Verification evidence

- Verification: `cargo test -p zlf-core -p zlf-storage -p zlf-index` → 19 core, 18 index unit, 5 temporal integration, 1 storage unit, and 18 storage integration tests passed → **pass**.
- Verification: `cargo clippy -p zlf-core -p zlf-storage -p zlf-index --all-targets -- -D warnings -W clippy::too_many_lines` → completed with no warnings → **pass**.
- Verification: `python3 scripts/check-rust-size.py` → no violations → **pass**.
- Verification: `cargo fmt --all` and `git diff --check` → no formatting/whitespace errors → **pass**.

### Reversal and next dependency

The increment is additive and reversible by removing the new modules/exports; later increments may change `Node` or `Edge` bytes directly because no pre-release database compatibility is required. It was not committed because the worktree already contains unrelated user-staged files under `.agents/prompt-history/` and `data/ncbi-taxnomy/`.

I0 legacy fixture work was cancelled by the pre-release schema decision. The next unblocked increment is I2 atomic canonical mutation/outbox. Backend stages remain dependency-gated, not blocked by an unresolved design decision.
