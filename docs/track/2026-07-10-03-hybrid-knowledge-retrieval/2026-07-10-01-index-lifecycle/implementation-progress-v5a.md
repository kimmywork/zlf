# Stage 01 Implementation Progress v5a

## Increment I5a — Profiles, chunkers, and manifest foundations

**Status:** completed on 2026-07-11

### Delivered

- Storage-neutral atomic projection configuration commit ordered in the mutation outbox.
- Immutable persisted `IndexProfileArtifact` store, activation pointer, listing, idempotent artifact put, and reopen behavior.
- Deterministic whole-field, paragraph/heading, fixed mixed-language token-window, and explicit adapter chunk handling.
- SHA-256 content fingerprints, stable chunk IDs, UTF-8 byte source ranges, and content-bearing index documents.
- Per-entity/profile document manifest validation and deterministic upsert/delete reconciliation with stale-version rejection.

### Verification

- Verification: chunking goldens → English/Chinese mixed windows, headings/paragraphs, UTF-8 ranges, explicit identity, invalid options pass → **pass**.
- Verification: immutable profile store tests → put/idempotence/conflict/activate/reopen/list pass → **pass**.
- Verification: manifest tests → update/add/delete/idempotence/stale suppression pass → **pass**.
- Verification: focused clippy, format, Rust size, and diff checks → **pass**.

### Next

I5b adds JSON and Prolog directive lowering through this single profile store and persists manifest adapters.
