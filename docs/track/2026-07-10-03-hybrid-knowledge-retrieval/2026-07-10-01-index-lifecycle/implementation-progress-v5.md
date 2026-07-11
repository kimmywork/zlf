# Stage 01 Implementation Progress v5

## Increment I5 — Immutable profiles, chunks, and manifests

**Status:** completed on 2026-07-11

### Delivered

- Content-addressed immutable profile artifacts with canonical SHA-256 validation.
- Atomic profile artifact/activation commits ordered with graph mutations in the durable outbox.
- Rust, JSON-over-STDIO, and Prolog `index_profile/3` plus `activate_index_profile/2` lowering through one store.
- Deterministic whole-field, paragraph/heading, fixed mixed-language token-window, and explicit adapter chunks.
- Stable chunk identity, content fingerprint, source byte range, ordinal, profile, and content payload.
- Durable per-entity/profile manifests with deterministic reconciliation, idempotence, delete sets, stale-version rejection, and reopen.

### Verification

- Verification: profile tests → immutable conflicts, canonical hashes, activation order, JSON/Prolog lowering, list, and reopen pass → **pass**.
- Verification: chunking/manifest tests → mixed language, UTF-8 boundaries, explicit chunks, update/delete/idempotence/stale behavior pass → **pass**.
- Verification: CLI profile integration → put/activate/list pass → **pass**.
- Verification: full workspace clippy, format, size, and diff gates → **pass**.
- Verification: non-CLI workspace suite after I5 → all deterministic tests pass → **pass**.

### Next

I6 implements durable target jobs, leases/retry/dead/stale states, contiguous watermarks, outbox compaction, and a deterministic fake target.
