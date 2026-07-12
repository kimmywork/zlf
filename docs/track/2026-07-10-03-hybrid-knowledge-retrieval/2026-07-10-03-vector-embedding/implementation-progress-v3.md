# Stage 03 Implementation Progress v3

## Increment V2a — Durable embedding job state machine

**Status:** completed on 2026-07-11; provider batching and exact-store publication continue in V2b.

### Delivered

- Generation/model/document-scoped durable embedding jobs with source version, fingerprint, expected dimension, attempts, lease/retry/completion timestamps, and redacted error class.
- Fingerprint/source-version dedupe and replacement of changed work.
- Deterministic bounded claim ordering, expired-lease recovery, delayed retry, attempt-limit dead letter, completion, stale acknowledgement, and reopen.
- Error classes are truncated to 128 characters; source text and credentials are absent from the durable job envelope.

### Verification

- `cargo test -p zlf-query --test embedding_jobs`
- `cargo clippy -p zlf-index -p zlf-query --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`

### Next

V2b adds the batch provider boundary and worker: load manifest text outside WAM execution, transform per model profile, suppress stale source versions, validate/provider-normalize vectors, atomically publish to `ExactVectorStore`, and crash-replay with a deterministic fake provider.
