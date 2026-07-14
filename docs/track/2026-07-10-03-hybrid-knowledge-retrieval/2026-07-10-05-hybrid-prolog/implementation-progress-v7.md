# Stage 05 implementation progress v7

## Increment

H6a adds a reproducible public-judgment lexical/vector/hybrid quality baseline on the prepared SciFact subset.

## Delivered

- Added `zlf-query` release example `scifact_h6_benchmark`.
- Loads the deterministic 1,000-document/100-query subset and official qrels.
- Builds real Tantivy BM25 documents and generation/model-scoped exact RocksDB vectors.
- Generates document and query embeddings through Ollama OpenAI-compatible batches using `bge-m3:latest` at 1,024 dimensions.
- Compares BM25, exact vector, and fixed RRF `k=60` on identical judgments.
- Reports MRR, nDCG@10, Recall@10/100, bounded candidate/answer counts, p50/p95/p99, fusion-only and end-to-end hybrid latency, embedding latency, RSS, and disk.
- Records dataset checksums, machine, commit/dirty state, analyzer/chunking/model/backend, and all limits.

## Result

BM25 achieved MRR 0.816469 and nDCG@10 0.821813. Exact vector achieved MRR 0.760906 and nDCG@10 0.782273. RRF achieved MRR 0.801855 and nDCG@10 0.816503.

RRF increased Recall@10 from BM25's 0.880667 to 0.904667 and Recall@100 from 0.966667 to 0.990000, but reduced MRR by 0.014614 and nDCG@10 by 0.005310. The report therefore does not claim fusion is generally better.

Retrieval p99 was 1.78 ms BM25, 4.39 ms exact vector, and 5.79 ms hybrid end-to-end. The fused candidate union averaged 168.56 documents and materialized answers remained bounded to 100.

## Evidence

- `research/scifact-h6-local-2026-07-14.json`
- `research/scifact-h6-local-2026-07-14.md`

## Verification

```bash
cargo fmt --all
python3 scripts/check-rust-size.py
cargo clippy -p zlf-query --example scifact_h6_benchmark -- \
  -D warnings -W clippy::too_many_lines
cargo build --release -p zlf-query --example scifact_h6_benchmark
target/release/examples/scifact_h6_benchmark \
  data/benchmarks/scifact/h6-1000d-100q-v1
python3 -m json.tool research/scifact-h6-local-2026-07-14.json
git diff --check
```

## Remaining H6

Generate and run EnterpriseKB-v1 1K/10K workloads for ACL graph/rule filtering, temporal constraints, permission mutation, stale-result prevention, filter/top-k ordering, selectivity, bounded materialization, latency, RSS, and disk. H7 cumulative review follows.
