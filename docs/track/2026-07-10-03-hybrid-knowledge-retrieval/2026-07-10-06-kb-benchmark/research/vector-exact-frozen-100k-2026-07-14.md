# Frozen 100K × 1024 exact-vector baseline — 2026-07-14

## Scope

Retrieval-only, sequential, single-process exact RocksDB baseline over the immutable vector dataset. No Ollama, HTTP, text transformation, or query embedding is involved.

Configuration:

```text
100,000 documents
1,024 f32 dimensions
100 measured queries
cosine, L2-normalized
top-k 10 and 100
unfiltered, approximately 10%, approximately 1% metadata filters
```

## Correctness

All 100 byte-identical self queries returned their referenced document at rank 1. Filtered hits were independently checked against immutable document groups.

## Results

| workload | p50 | p95 | p99 | sequential QPS |
|---|---:|---:|---:|---:|
| unfiltered top-10 | 637.69 ms | 717.98 ms | 742.94 ms | 1.56 |
| unfiltered top-100 | 650.80 ms | 733.77 ms | 795.36 ms | 1.51 |
| 10% filter top-10 | 387.78 ms | 425.30 ms | 483.99 ms | 2.55 |
| 10% filter top-100 | 388.59 ms | 420.98 ms | 530.73 ms | 2.54 |
| 1% filter top-10 | 362.32 ms | 377.39 ms | 402.70 ms | 2.75 |
| 1% filter top-100 | 363.38 ms | 381.65 ms | 398.81 ms | 2.74 |

Fresh-reader top-10 over ten queries measured 674.04 ms p50 and 680.76 ms p99. Reopen took 60.36 ms.

Resources:

```text
backend build/write: 0.94 s
index size:          451,322,126 bytes (430.4 MiB)
peak RSS:            293,076,992 bytes (279.5 MiB)
```

The build number measures RocksDB batch writes returning successfully; it is not presented as a forced full-compaction or OS-cold durability benchmark.

## Interpretation

The exact backend remains a useful correctness oracle and fallback, but unfiltered 100K × 1024 p99 near 0.75–0.80 seconds is too slow for a low-latency interactive vector engine. Selective metadata filtering helps, but still remains around 0.40–0.53 seconds p99 because records must be scanned/deserialized before metadata rejection.

This is sufficient evidence to evaluate an ANN candidate backend. Any ANN result must consume the identical corpus/query/group files and report Recall@10/100 against exact results, plus build/reopen/update/delete, RSS, disk, and filtered behavior. Exact RocksDB is retained rather than replaced.
