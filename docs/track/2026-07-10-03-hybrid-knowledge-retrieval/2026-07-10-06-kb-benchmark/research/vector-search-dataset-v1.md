# Frozen vector search dataset v1

## Purpose

Provide one immutable, engine-neutral corpus for repeated exact and future ANN retrieval-only comparisons. No Ollama or HTTP call is involved in generation or query execution.

## Shape

```text
documents:       100,000
queries:           1,000
self queries:         100
dimension:           1,024
dtype:        little-endian f32
metric:               cosine
normalization:        L2
seed:             20260714
```

Files under ignored `data/benchmarks/vector-search-100k-1024-v1/`:

| file | bytes | SHA-256 |
|---|---:|---|
| `documents.f32le` | 409,600,000 | `e7ea455193781509c80ec55b1164cb600132ba802d8ce90f487133242650c53a` |
| `queries.f32le` | 4,096,000 | `2211eae939dd9d7a50a90eeb2763997e4d17e915259c79e329c86d8768f3c2a1` |
| `document-groups.u16le` | 200,000 | `7d5c8ec357b6f5ffb593aaff26dcdf7d5ab8532ca86120da7642be755da22be1` |
| `self-query-document-ids.u32le` | 400 | `12f91d105ee44236f7919c5a308e0baf81542f21b639bbfdd46ddc6486638bf5` |

Total local size is approximately 406 MiB.

## Generation

```bash
python3 scripts/prepare-vector-search-dataset.py
```

The generator uses versioned SplitMix64 high-24-bit uniform values, f64 norm accumulation, and final f32 little-endian storage. It writes into a temporary directory, computes all checksums, atomically publishes the directory, and verifies it. A later invocation verifies the existing files and does not regenerate them unless `--force` is explicit.

The first 100 queries are exact copies of deterministic document IDs `(query_index * 997) % 100000`; their maximum component difference is zero and they provide a self-nearest-neighbor correctness gate. The remaining 900 queries use a separate PRNG domain. Every document also has an immutable group in `[0,1000)` for unfiltered, 10%, and 1% filter workloads.

## Validation

- All sampled document/query norms are 1 within f32 precision.
- Every component is finite.
- Self-query vectors are byte-equivalent to their referenced document vectors.
- A small binary checksum golden locks the generator algorithm in unit tests.
- The full dataset was regenerated once, then a second invocation checksum-verified and reused it.

## Benchmark policy

Every vector backend consumes these same bytes, IDs, groups, queries, top-k values, and filter definitions. Backend-specific preprocessing/build artifacts are separate and disposable. Exact RocksDB supplies correctness results; ANN candidates, if later added, must report Recall@k against exact results from this frozen dataset.
