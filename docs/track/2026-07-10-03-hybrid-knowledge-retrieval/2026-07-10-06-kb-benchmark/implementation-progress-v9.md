# Stage 06 implementation progress v9

## Increment

Prepare and freeze the reusable 100K × 1024 retrieval-only vector dataset before benchmarking exact or ANN engines.

## Delivered

- Added `scripts/prepare-vector-search-dataset.py`.
- Generated immutable little-endian f32 document/query files, u16 filter groups, and u32 self-query IDs.
- Added atomic publication, full size/checksum verification, and default reuse rather than regeneration.
- Added a frozen small binary checksum golden to detect accidental PRNG/normalization/serialization changes.
- Recorded complete full-dataset checksums and benchmark policy in `research/vector-search-dataset-v1.md`.

## Dataset

- 100,000 L2-normalized document vectors.
- 1,000 L2-normalized query vectors.
- 1,024 dimensions and cosine metric.
- 100 byte-identical self queries for correctness.
- 900 independent deterministic queries for latency/QPS.
- 1,000 deterministic group values supporting 10% and 1% filter workloads.
- Approximately 406 MiB local size.

## Verification

```bash
python3 scripts/prepare-vector-search-dataset.py
python3 scripts/prepare-vector-search-dataset.py  # verifies and reuses
python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v
```

Sampled norms are 1 within f32 precision, all values are finite, self-query vectors are byte-equivalent to referenced documents, and all full-file SHA-256 values match the recorded manifest.

## Next

Build exact RocksDB records from this frozen corpus once, then run retrieval-only top-10/top-100 unfiltered, 10%, and 1% filter workloads. ANN, if later justified, must consume the same corpus/query/filter bytes and compare Recall@k against exact results.
