# Frozen 100K × 1024 HNSW evaluation — 2026-07-14

## Scope

Retrieval-only, sequential, single-process `hnsw_rs 0.3.4` evaluation over the exact same immutable corpus, query, group, and ID bytes as the exact RocksDB baseline. No Ollama, HTTP, text transformation, or query embedding is involved.

Configuration:

```text
100,000 documents; 1,024 normalized f32 dimensions; cosine distance
100 measured queries; top-k 10 and 100
M=48; ef_construction=400; max_layer=16
ef_search=128, 256, 512, 1024, 2048
unfiltered, 10% group filter, 1% group filter
```

The exact RocksDB backend generated every top-k oracle. Recall is intersection Recall@k against those exact answers, not only self-query accuracy.

## Quality/latency trade-off

### Recommended quality setting: ef_search=2048

| workload | Recall@k | p50 | p95 | p99 | sequential QPS |
|---|---:|---:|---:|---:|---:|
| unfiltered top-10 | 0.9810 | 96.20 ms | 110.35 ms | 137.17 ms | 10.18 |
| unfiltered top-100 | 0.9801 | 95.01 ms | 98.13 ms | 106.43 ms | 10.46 |
| 10% filter top-10 | 0.9970 | 376.99 ms | 388.10 ms | 394.37 ms | 2.65 |
| 10% filter top-100 | 0.9959 | 382.50 ms | 393.56 ms | 399.17 ms | 2.61 |
| 1% filter top-10 | 0.9950 | 255.62 ms | 266.36 ms | 275.57 ms | 3.90 |
| 1% filter top-100 | 0.9975 | 228.24 ms | 272.17 ms | 294.51 ms | 4.24 |

Against exact RocksDB, unfiltered p99 improved from 742.94/795.36 ms to 137.17/106.43 ms (5.4×/7.5×), while retaining 0.9810/0.9801 Recall@10/100. Filtered p99 improved by approximately 1.2×–1.5× at this setting.

### Lower-latency setting: ef_search=1024

| workload | Recall@k | p99 | sequential QPS |
|---|---:|---:|---:|
| unfiltered top-10 | 0.9480 | 86.85 ms | 13.58 |
| unfiltered top-100 | 0.9299 | 81.30 ms | 13.65 |
| 10% filter top-10 | 0.9960 | 250.95 ms | 4.34 |
| 10% filter top-100 | 0.9951 | 430.42 ms | 3.87 |
| 1% filter top-10 | 0.9950 | 488.65 ms | 2.33 |
| 1% filter top-100 | 0.9975 | 498.01 ms | 3.05 |

The complete JSON preserves all five `ef_search` settings. Lower settings are faster but fall below 0.90 unfiltered Recall@10/100; they are not selected as the quality-oriented default.

## Identity, durability, and lifecycle

- The durable mapping contains 100,000 validated entries from HNSW `usize` IDs to canonical `doc-NNNNNN` IDs.
- Dataset checksums, generation, model profile/version, mapping schema, and HNSW parameters are stored in the publication marker.
- Graph, vector data, mapping, build timing, and identity marker are built in a temporary directory and renamed as one immutable publication.
- Fresh reload restored the index in 817.27 ms; all 100 byte-identical self queries returned the canonical referenced ID at rank 1.
- A rebuild probe verified update, delete, insert, dump, and reload behavior.
- `hnsw_rs` has no in-place update/delete API. Mutations therefore require publishing a replacement immutable generation; exact RocksDB remains the source of truth and fallback.
- The crate dump encodes native-sized/native-endian details and parallel construction is not asserted byte-deterministic. A published dump is durable for this measured deployment, but cross-architecture portability and identical rebuild bytes are not claimed.

Resources:

```text
build:      549.12 s
index:      599,301,809 bytes (571.5 MiB, including canonical mapping)
peak RSS:   1,333,395,456 bytes (1,271.6 MiB)
reopen:     817.27 ms
```

Compared with exact RocksDB, this candidate uses about 1.33× disk and 4.55× peak RSS, and its full build is much slower. These are material deployment costs.

## Decision

Adopt `hnsw_rs` as the qualified ANN implementation candidate and retain `ef_search=2048` as the measured quality-oriented setting. Do **not** replace or remove exact RocksDB: it remains the correctness oracle, mutation source, and fallback. Do **not** route the production `ZlfDatabase` path to HNSW in this increment; production routing still needs generation-level rebuild orchestration and a tested corrupt/missing-ANN fallback at the facade boundary.

This completes the frozen-engine optimization evaluation without overstating it as a production cutover. The result is not comparable to semantic leaderboards because the corpus is generated high-dimensional data and the metric is ANN recall against the local exact oracle.
