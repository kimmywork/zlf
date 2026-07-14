---
status: done
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-05-hybrid-prolog/requirements-v1.md
---

# Stage 05 Solution Design v1: Hybrid Retrieval and Prolog Composition

## Scope and dependency

Stage 05 begins only after validated Stage 01 lifecycle and focused Stage 02–04 backend contracts exist. It preserves `ZlfDatabase -> WamRuntime -> CompositeFactProvider`, keeps `FactProvider` read-only, and adds no second evaluator.

## Retrieval contract

```text
RetrievalRequest { query text/vector/handle, modes, profiles, top_k, candidate_k,
  threshold, fields, model/analyzer generations, temporal filter,
  graph_filter_goal?, minimum_watermarks?, source exclusion, explain, budgets }
RetrievalHit { document/entity/field/chunk, fused rank/score,
  lexical rank/score?, vector rank/score?, generation/watermark,
  source range, provenance, explanation? }
```

All limits are explicit and validated. Stable document ID breaks ties. Core predicates receive explicit first-version bounded contracts; hybrid work uses `retrieve(Query, Options, Entity, Hit)`. No compatibility aliases are added.

## Preparation and fusion

The async query facade resolves profiles/watermarks and embeds literal semantic queries before WAM starts. It registers an immutable request-scoped prepared handle. Synchronous WAM calls with uncached literal text return a typed preparation error; no HTTP call occurs in the instruction loop.

Hybrid fusion uses reciprocal-rank fusion: `sum(1/(k+rank))`, with versioned default `k=60`. Missing retrievers contribute nothing. Raw BM25/cosine values are returned for diagnostics but never added. Fusion deduplicates by `IndexDocumentId`; optional entity aggregation is a separately named/requested policy.

## Filtering and planning

- **Bound/filter-first:** when an entity binding or selective graph/property/label plan is available, query backend bound-document lookup and score only allowed documents.
- **Retrieval-first:** page ranked candidates, execute the compiled WAM filter goal for each entity, and continue until accepted `top_k`, candidate budget, or exhaustion.

Planner output records mode, pushed bounds, page/candidate counts, rejected count, exhaustion, and whether exact filtered top-k is guaranteed. ACL-style filters are ordinary graph/rule predicates and are not a security boundary.

## Bounded answer production

The first functional implementation retains the existing materialized provider path but requires explicit backend `top_k`, candidate, page, and answer limits before results enter WAM. This keeps backtracking/cut behavior unchanged while bounding memory. Providers may internally page while building the bounded answer set.

A WAM-owned external cursor remains a deferred optimization. Add it only if bounded materialization cannot satisfy accepted limits; it then requires the full backtracking, nested choice, cut, once, exception, proof, and ownership test matrix before adoption.

## Proof and tabling

Proof leaves carry index kind, profile/generation, document ID, rank/score, and fingerprint, never full source text. Retrieval is tableable only when the prepared handle, options, and exact index generation/watermark dependencies are part of the table key/dependency set. Unsupported live/latest-watermark combinations fail explicitly as non-tableable. Generation activation and relevant worker publication invalidate dependent tables through existing selective invalidation.

## Consistency and errors

Minimum-watermark waits happen before WAM execution. Timeout reports committed source sequence and pending targets. Incompatible model/analyzer/schema generations fail rather than mixing. Candidate-budget exhaustion returns partial/exhaustion metadata and does not silently claim exact top-k.

## Verification

- Facade oracle tests for lexical/vector/RRF ties and provenance.
- Fake cursor WAM matrix for ordering, paging, cut/backtracking/once/exceptions/proof and peak answers.
- Goal-order tests for bound lookup and progressive filtering.
- Graph/property/rule/event/validity joins, permission mutation, generation activation, table invalidation, restart.
- Quality comparison on identical judgments and latency/candidate/materialization reports at approved tiers.

## Risks and rollback

- **Bounded materialization (medium):** enforce limits at every backend/provider boundary and report exhaustion; defer WAM cursor complexity until needed.
- **Filtered top-k correctness (high):** report guarantee/exhaustion and use progressive paging.
- **Stale tables (high):** default unsupported combinations to non-tableable rather than cache incorrectly.
- **Rollback:** disable `retrieve/4`, retain existing predicates and materialized provider path, reactivate prior index generations.
