# Exact vector local baseline — 2026-07-11

## Method

The release benchmark atomically builds deterministic normalized 64-dimensional vectors, executes 100 self-nearest-neighbor queries at top-10, replaces 100 records, reopens RocksDB, repeats queries, and records latency, RSS, and disk.

```bash
cargo run --release -p zlf-index --example vector_exact_benchmark -- 1000
cargo run --release -p zlf-index --example vector_exact_benchmark -- 10000
```

Machine: Apple M2 Pro, 32 GiB.

| Vectors | Build | Replace 100 | Warm p99 | Fresh-reader p99 | RSS | Disk | MRR / Recall@10 |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1,000 | 0.97 ms | 0.17 ms | 0.63 ms | 0.69 ms | 17.9 MB | 465 KB | 1.0 / 1.0 |
| 10,000 | 8.74 ms | 0.22 ms | 5.86 ms | 6.19 ms | 59.9 MB | 3.90 MB | 1.0 / 1.0 |

## Limits

- These deterministic vectors verify exact-search correctness and regression behavior, not semantic relevance.
- The 64-dimensional fixture keeps routine local evidence fast; default Ollama `bge-m3` is separately verified at 1024 dimensions.
- Fresh reader means close/reopen in one process, not an OS page-cache cold run.
- ANN Recall@k is not reported because ANN was explicitly deferred; exact recall is the oracle baseline.
