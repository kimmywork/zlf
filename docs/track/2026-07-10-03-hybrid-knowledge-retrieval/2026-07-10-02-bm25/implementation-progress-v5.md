# Stage 02 Implementation Progress v5

## Increment B4 — Bounded filters, batch updates, and local evidence

**Status:** completed on 2026-07-11.

### Delivered

- Atomic Tantivy `DocumentChanges` batches avoid one commit/reload per chunk and are used by the lifecycle target.
- Optional language metadata is profile/document/schema scoped and queryable together with field filters through `search_document_top_k_filtered` and `search_bm25_filtered`.
- Production Tantivy scores are differentially checked against the independent formula fixture.
- A reproducible release benchmark records 1K/10K build and replace throughput, warm/fresh-reader p50/p95/p99, RSS, disk, MRR, nDCG@10, and Recall@10/100 as JSON.
- Generous first regression budgets are frozen in `research/bm25-local-2026-07-11.json`.

### Evidence

- 1K: 184.3 ms build; 193.4 ms replace-100; 12 µs warm p99; 14 µs fresh-reader p99; 154.1 MB RSS; 66.4 KB disk.
- 10K: 200.0 ms build; 210.7 ms replace-100; 13 µs warm p99; 12 µs fresh-reader p99; 158.5 MB RSS; 501.8 KB disk.
- Both deterministic tiers: MRR, nDCG@10, Recall@10, and Recall@100 = 1.0.

### Scope note

This is a functional synthetic regression baseline, not a general-quality claim. EnterpriseKB/SciFact/BEIR and true OS-cold orchestration remain in Stage 06 under the accepted function-first decision. The optional 100K tier was not needed for first functional acceptance.
