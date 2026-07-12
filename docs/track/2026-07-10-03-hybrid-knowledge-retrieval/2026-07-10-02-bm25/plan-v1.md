---
status: done
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-02-bm25/requirements-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-02-bm25/solution-design-v1.md
---

# Stage 02 Plan v1: BM25 Correctness and Scale

## Dependency

```text
Stage 01 contracts/generations -> B0 oracle/contracts -> B1 Tantivy integration
 -> B2 implementation -> B3 lifecycle/reopen
 -> B4 quality/scale -> B5 review
```

## B0 — Scoring and analyzer oracle (medium)

Freeze chunk-as-document semantics, BM25 config/query/hit/explanation contracts, tiny hand-calculated corpora, independent Python scorer, and multilingual analyzer goldens.

**Exit:** formula, tolerance, token stream, ties, and invalid options are executable without a production backend.

## B1 — Tantivy integration (medium)

Add Tantivy behind the common lexical contract, wire the versioned analyzer, generation directory, replace/delete, reopen, and top-k behavior. Keep independent score/token fixtures.

**Exit:** Tantivy satisfies the functional scoring, analyzer, lifecycle, and reopen contract.

## B2 — Production scoring and bounded retrieval (high)

Implement generation-scoped statistics/postings, field weights, top-k heap, candidate budget, deterministic ties, explanations, and explicit incompatible-schema rejection.

**Exit:** differential scorer/ranking tests pass within frozen tolerance.

## B3 — Lifecycle integration (high)

Consume Stage 01 manifests/jobs for idempotent replace/delete/replay, validation, activation, rollback, and fresh-process reopen.

**Exit:** corpus/DF/length statistics remain exact after update/delete/crash replay.

## B4 — Quality and local scale (medium)

Run 1K/10K and at most 100K EnterpriseKB/SciFact tiers; record MRR, nDCG, Recall, throughput, percentiles, RSS, disk, candidates, cold/warm state. Freeze regression budgets from accepted evidence.

## B5 — Review and delivery evidence

Run focused/workspace gates, cumulative implementation review, and acceptance record. Backend work cannot be called BM25-correct before B2/B3 or production-ready before B4.

## Acceptance mapping

- real statistics/formula/top-k/ties: B0/B2
- multilingual versioned analysis: B0/B1/B2
- replace/delete/rebuild: B3
- independent quality/performance: B0/B4
- clean prototype replacement: B2/B3

## Stop conditions

Return to design only if Tantivy cannot satisfy required scoring/analyzer lifecycle. Keep the previous active generation on all failures; performance optimization is deferred until functionality is stable.
