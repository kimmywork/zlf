# Stage 06 implementation progress v10

## Increment

Run the first exact RocksDB engine baseline against the frozen 100K × 1024 corpus.

## Delivered

- Added `vector_frozen_benchmark` release example.
- Builds/reuses a dataset-identity-scoped exact RocksDB backend.
- Streams immutable f32 document vectors in bounded batches.
- Runs top-10/top-100 unfiltered, 10%, and 1% group-filter workloads.
- Checks all self-query top-1 identities and every filtered hit's group.
- Reports warm/fresh-reader percentiles, sequential QPS, build/reopen, RSS, disk, and backend reuse.
- Added shared-schema report packaging.

## Result

- 100/100 self queries correct at rank 1.
- Unfiltered p99: 742.94 ms top-10; 795.36 ms top-100.
- 10% filter p99: 483.99/530.73 ms.
- 1% filter p99: 402.70/398.81 ms.
- Sequential unfiltered QPS: 1.51–1.56.
- Fresh-reader top-10 p99: 680.76 ms.
- Index: 430.4 MiB; peak RSS: 279.5 MiB.

## Decision

Exact RocksDB remains the correctness oracle and fallback, but the measured unfiltered 100K × 1024 latency justifies evaluating an ANN candidate backend on the same frozen bytes. No Ollama or end-to-end benchmark is involved.

## Evidence

- `research/vector-exact-frozen-100k-2026-07-14.json`
- `research/vector-exact-frozen-100k-2026-07-14.md`

## Next

Implement the smallest durable HNSW experiment with canonical ID mapping, generation/model identity, persistence/reopen, and Recall@k comparison against this exact result. Do not cut over the production backend until update/delete/rebuild behavior and quality gates pass.
