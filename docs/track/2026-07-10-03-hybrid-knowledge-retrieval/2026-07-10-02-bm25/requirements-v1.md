---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-10
version: 1
---

# Stage 02 Requirements: BM25 Correctness and Scale

## Goal

Replace token-frequency accumulation with a real, update-safe, field/chunk-aware BM25 implementation and establish lexical quality/performance evidence.

## Requirements

- Maintain corpus document count, per-document length, average length, per-term document frequency, and postings with term frequency.
- Implement the documented BM25 formula with configurable/versioned `k1` and `b`.
- Search accepts top-k and optional field/language filters and uses bounded candidate/ranking memory.
- Define one logical document for statistics—chunk, field, or node—before implementation.
- Preserve Chinese/English analysis with analyzer version and deterministic tokenization tests.
- Replace/update/delete removes obsolete postings and updates corpus statistics exactly once.
- Tie-break equal scores deterministically by stable document identity.
- Provide score components/explanation for diagnostics.
- Prototype token-count index data is discarded; new generations contain only real BM25 data.

## Verification

- Hand-calculated miniature corpora verify TF, IDF, length normalization, updates, and deletes.
- Differential tests compare scores/ranks with an independent reference implementation or script within a specified floating-point tolerance.
- Retrieval datasets report MRR, nDCG@10, Recall@10/100, index build/update throughput, p50/p95/p99 query latency, peak RSS, and index size.
- Local tiers cover 1K–10K smoke and at most 100K chunks on the current M2 Pro/32-GiB machine.
- Warm and fresh-process/cold behavior are reported separately.

## Non-goals

- Phrase/proximity, fuzzy search, stemming for every language, or learned sparse retrieval unless evidence makes them necessary.
- Direct score addition with cosine similarity.

## Confirmed backend policy

Use Tantivy as the initial mainstream embedded backend with a versioned Chinese/English analyzer adapter. Keep the backend contract replaceable and retain independent correctness tests. Defer a custom RocksDB alternative until stable functionality demonstrates a need.
