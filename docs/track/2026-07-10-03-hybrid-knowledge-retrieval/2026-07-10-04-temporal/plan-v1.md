---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-04-temporal/requirements-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-04-temporal/solution-design-v1.md
---

# Stage 04 Plan v1: Temporal Semantics and Indexes

## Dependency

```text
Stage 01 lifecycle -> T0 semantic/encoding oracle -> T1 event index
 -> T2 validity indexes -> T3 lifecycle/provider/planner
 -> T4 skew/scale decision -> T5 review
```

## T0 — Contract and boundary oracle (medium)

Freeze event/validity records, UTC parser, half-open validation, signed-microsecond codec, independent filter oracle, and exhaustive boundary fixtures.

## T1 — Event-time index (medium)

Implement by-time/by-entity generation keys and bounded seek APIs for UTC day, range, before, and after. Preserve duplicate instants and stable record ordering.

## T2 — Valid-time indexes (high)

Implement by-start/by-end/open-end/by-entity keys, `valid_at`, overlap, endpoint estimates/intersection, candidate counts, and bounded paging.

## T3 — Lifecycle and Prolog/planner integration (high)

Extract only profile-declared fields; reconcile update/delete/replay; validate/reopen generations; implement approved predicates, provenance, and access-path reporting without changing WAM architecture.

## T4 — Oracle and local scale (medium)

Differential-test all records and run 1K/10K/100K uniform/skew/open histories with latency, candidates, throughput, RSS, and disk. Add buckets/another derivative only through a reviewed evidence-backed change note.

## T5 — Review and delivery evidence

Run focused/workspace gates and cumulative implementation/acceptance reviews.

## Acceptance mapping

- explicit event/valid semantics and parsing: T0
- seek-based event queries: T1
- containment/overlap/open ends: T2
- lifecycle/provenance/composition: T3
- skew correctness/performance: T4

## Stop conditions

Return to design if ordered encoding fails an instant boundary or normal bound queries require full scans. Preserve prior generation on rebuild/validation failure.
