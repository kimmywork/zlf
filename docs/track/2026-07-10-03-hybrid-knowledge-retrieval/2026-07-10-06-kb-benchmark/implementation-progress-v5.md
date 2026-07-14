# Stage 06 implementation progress v5

## Increment

S1 completion: batch BM25 rebuild publication, add an explicit stale-only worker regression, and rerun the lifecycle baseline.

## Delivered

- BM25 generation rebuild now computes all desired/retired manifests, merges changes by canonical document identity, applies one atomic Tantivy `DocumentChanges` batch, and publishes manifests only after backend success.
- Retired or no-longer-matching profile manifests are cleared during rebuild.
- Added a 96-stale-job fixture followed by current ready work; one worker call now marks all stale jobs and still publishes the later current document.
- Reran the complete 1K mutation/retry/watermark/rebuild/rollback/reopen workload.

## Result

The same lifecycle oracle remains exact:

- 1,000 initial embeddings and 15 mutation embeddings published.
- 32 injected retry jobs recovered.
- 20/20 mutations match before and after reopen.
- Minimum watermarks reached; missing target timed out explicitly.
- BM25 rebuild/rollback and post-reopen search passed.

BM25 rebuild batching reduced complete initial profile activation/build from approximately 150.4 seconds to 1.02 seconds. Peak RSS fell from approximately 262 MiB to 236 MiB. Mutation/re-embedding remained approximately 1.86 seconds.

## Verification

```bash
cargo test -p zlf-query --test bm25_lifecycle --test embedding_worker_v2
cargo clippy -p zlf-query --all-targets -- \
  -D warnings -W clippy::too_many_lines
cargo fmt --all -- --check
python3 scripts/check-rust-size.py
git diff --check
```

## Next

S1 is accepted for its approved generated tiers. Continue with S2 primary-source license/schema/checksum research for FiQA, MIRACL en/zh, HotpotQA/KILT, and LoCoMo/LongMemEval before adopting public datasets.
