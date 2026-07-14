# SciFact H6 local baseline — 2026-07-14

## Scope

This is the first real relevance baseline for Stage 05 H6. It uses the same 1,000-document, 100-query SciFact subset and official qrels for BM25, exact vector, and RRF hybrid. It is not a full WAM/ACL benchmark; EnterpriseKB owns graph/rule/temporal/filter correctness in the next H6 increment.

## Reproduction

```bash
cargo run --release -p zlf-query --example scifact_h6_benchmark -- \
  data/benchmarks/scifact/h6-1000d-100q-v1 \
  > /tmp/zlf-scifact-h6.json
```

The run uses Ollama `bge-m3:latest`, 1,024 dimensions, exact cosine vectors, whole-abstract body documents, candidate limit 100, answer limit 100, and RRF `k=60`. Embedding build and query embedding time are reported separately from retrieval latency.

Dataset manifest and file checksums are recorded in the adjacent JSON report.

## Quality

| retriever | MRR | nDCG@10 | Recall@10 | Recall@100 |
|---|---:|---:|---:|---:|
| BM25 | 0.816469 | 0.821813 | 0.880667 | 0.966667 |
| exact vector | 0.760906 | 0.782273 | 0.881000 | 0.970000 |
| RRF hybrid | 0.801855 | 0.816503 | 0.904667 | 0.990000 |

Hybrid versus BM25:

- MRR: -0.014614
- nDCG@10: -0.005310
- Recall@10: +0.024000
- Recall@100: +0.023333

Hybrid improves recall on this subset but does not improve BM25 MRR or nDCG@10. Therefore the evidence does **not** justify claiming that hybrid is generally better. The result is a useful regression baseline and confirms that the fixed RRF implementation produces a measurable, non-trivial trade-off.

## Local performance

- BM25 build: approximately 0.24 s.
- Document embedding build: approximately 110.2 s.
- Query embedding: approximately 19.0 s.
- Peak RSS: approximately 114.3 MiB.
- BM25/vector temporary index footprint: approximately 5.95 MiB.
- Retrieval p50/p95/p99:
  - BM25: 1.06 / 1.28 / 1.78 ms.
  - exact vector: 3.95 / 4.17 / 4.39 ms.
  - hybrid end-to-end (lexical + vector + fusion): 5.11 / 5.46 / 5.79 ms.
  - fusion-only: 0.08 / 0.12 / 0.16 ms.

Each retriever returned at most 100 candidates and at most 100 answers. The fused input union averaged 168.56 candidates per query; the result is bounded to 100 answers.

## Interpretation and next step

This report is a real public-judgment quality baseline, unlike the earlier synthetic BM25/vector correctness fixtures. It does not cover ACL, graph/rule joins, temporal filters, mutations, stale tables, or WAM materialization. Continue H6 with deterministic EnterpriseKB 1K/10K generation and those composition workloads before H7 acceptance.
