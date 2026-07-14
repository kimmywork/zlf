# Stage 06 implementation progress v3

## Increment

S1b prerequisite: expose durable embedding job-state metrics through the database facade so lifecycle benchmarks can report pending/leased/retry/dead/completed/stale counts without opening a second storage handle.

## Delivered

- Added `EmbeddingJobStore::state_counts()` with stable state names.
- Added `ZlfDatabase::embedding_job_state_counts()`.
- Kept the result aggregate-only; no document text, provider payload, or secret is exposed.
- Added lifecycle coverage confirming completed-job accounting.

## Verification

```bash
cargo fmt --all
python3 scripts/check-rust-size.py
cargo test -p zlf-query --test vector_lifecycle
cargo clippy -p zlf-query --all-targets -- \
  -D warnings -W clippy::too_many_lines
git diff --check
```

## Next

Use the facade in the S1b lifecycle runner to record retry, stale suppression, completion, reopen, mutation, watermark, and generation transitions.
