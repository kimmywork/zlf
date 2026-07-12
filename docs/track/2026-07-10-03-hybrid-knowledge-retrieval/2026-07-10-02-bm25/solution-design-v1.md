---
status: done
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-02-bm25/requirements-v1.md
---

# Stage 02 Solution Design v1: BM25 Correctness and Scale

## Scope and dependency

Stage 02 consumes Stage 01 `IndexDocumentId`, profiles, manifests, generations, jobs, and watermarks. It cannot implement a private document identity or activation mechanism. A logical BM25 document is exactly one indexed chunk in one field; corpus statistics are partitioned by profile generation, analyzer, and field. Node-level results are a later facade aggregation, not the scoring unit.

## Design principles

1. Correct replacement/delete and independently reproducible scores precede optimization.
2. Old token-count data is schema-incompatible and rebuilt into a new generation.
3. Search is bounded by explicit `top_k` and stable document-ID tie-breaking.
4. Analyzer identity is part of the generation contract.

## Backend choice

Use Tantivy as the initial production backend because it is a mainstream, maintained embedded Rust text engine with BM25, bounded top-k search, deletion, and persistent index support. zlf supplies a versioned Chinese/English tokenizer adapter and keeps the backend behind `LexicalBackend`. A custom RocksDB postings engine is deferred unless Tantivy later proves functionally incompatible.

## Contracts

```text
Bm25Config { schema, analyzer, k1, b, field_weights }
LexicalQuery { text, top_k, fields?, language?, generation, explain }
LexicalHit { document_id, score, rank, field, generation, explanation? }
LexicalBackend { build, reconcile, search, explain, validate, stats }
```

Defaults are versioned (`k1=1.2`, `b=0.75`) rather than implicit. Validation requires finite `k1 > 0`, `0 <= b <= 1`, nonempty top-k, and compatible profile/analyzer/schema generation.

BM25 uses `idf(t)=ln(1 + (N-df+0.5)/(df+0.5))` and the standard length-normalized TF factor. Field weights multiply field-local scores; statistics are not accidentally shared across incompatible fields. Stable canonical `IndexDocumentId` bytes break equal-score ties.

## Analyzer

`unicode_jieba_v1` performs deterministic Unicode normalization specified by fixtures, Jieba segmentation for Chinese, lowercased alphanumeric English token extraction, and removes empty tokens. Dictionary/version/checksum are generation metadata. Analyzer changes always rebuild; no in-place reinterpretation occurs.

## Lifecycle and bounded search

Stage 01 manifests produce exact old/new document deltas. Reconcile removes old postings/statistics before adding a replacement exactly once and is idempotent by source version/fingerprint. Tombstones remove all chunk postings. Generation validation recomputes document count, field token totals, and sampled DF from canonical documents.

Search gathers postings only for analyzed query terms, accumulates candidates in a bounded/request-budget map, and retains top-k in a heap. If a candidate budget is reached, the response reports truncation; it never silently claims exact top-k. Explanations contain query terms, TF, DF, IDF, document/average length, field weight, and score components.

## Verification and evidence

- Hand-calculated tiny corpora and an independent Python scorer with explicit floating tolerance.
- Analyzer golden files for Chinese, English, mixed text, punctuation, and Unicode normalization.
- Replace/delete/replay/reopen/generation tests through the Stage 01 fake/coordinator seam.
- Backend spike at 10K, followed after selection by 1K/10K/100K quality/performance tiers.
- SciFact/EnterpriseKB reports MRR, nDCG@10, Recall@10/100, p50/p95/p99, QPS, RSS, disk, and cold/warm state.

## Risks and rollback

- **Backend score drift:** validate against the common oracle; reject a candidate that cannot expose or reproduce required scoring.
- **Analyzer mismatch:** generation checksum fails closed and triggers rebuild.
- **High candidate memory:** enforce budgets and report truncation; optimize posting traversal only with profiles.
- **Rollback:** atomically reactivate the previous generation. Prototype index stays readable only until replacement validation, never as BM25 evidence.
