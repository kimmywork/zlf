---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-05-hybrid-prolog/requirements-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-05-hybrid-prolog/solution-design-v1.md
---

# Stage 05 Plan v1: Hybrid Retrieval and Prolog Composition

## Dependency

```text
Stages 01–04 contracts -> H0 facade/RRF oracle -> H1 prepared queries
H0 -> H2 bounded provider answers -> H3 index paging
H1 + H3 -> H4 filter planning/retrieve/4
 -> H5 proof/tables/consistency -> H6 quality/scale -> H7 review
```

## H0 — Structured facade and fusion oracle (medium)

Freeze request/hit/options/error/provenance types, explicit budgets, RRF behavior, deterministic ties, and lexical/vector/fusion oracle fixtures.

## H1 — Prepared async retrieval (medium/high)

Resolve generations/watermarks and query embeddings before WAM; add request-scoped handles and typed synchronous preparation errors. Verify no provider HTTP call occurs during WAM execution.

## H2 — Bounded provider answers (medium)

Retain existing WAM choice points and enforce explicit backend candidate/page/answer limits before materialization. Verify backtracking, cut, once, proof, exhaustion reporting, and peak answer bounds. Defer a WAM-owned cursor until measurements show it is needed.

## H3 — Index paging and bound lookup (high)

Adapt BM25/vector/temporal providers to page/cursor and pushed entity/generation constraints. Preserve existing predicates with documented limits.

## H4 — `retrieve/4`, RRF, and graph/rule filters (high)

Implement lexical/vector/hybrid plans, filter-first and progressive retrieval-first execution, candidate budgets, exhaustion/guarantee metadata, graph/property/label/rule/temporal joins, and planner output.

## H5 — Proof, table dependencies, and consistency (high)

Add compact proof leaves, generation/watermark dependency identity, publication invalidation, explicit non-tableable errors, and pre-WAM minimum-watermark waits.

## H6 — Quality and local scale (medium)

Compare retrievers/fusion on identical judgments; test ACL-style permission mutation and top-k ordering; report candidates, selectivity, percentiles, peak answers, RSS, and quality.

## H7 — Review and delivery evidence

Run focused/workspace gates and cumulative implementation/acceptance reviews.

## Acceptance mapping

- options/provenance/fusion: H0/H4
- no remote embedding in WAM: H1
- bounded backtracking/cut: H2/H3
- graph/rule/time filtering and planning: H4
- proof/table/freshness: H5
- measured hybrid quality: H6

## Stop conditions

Return to design if bounded materialization cannot preserve functional limits or filtered top-k cannot report exactness. Keep existing predicates and prior generations rather than ship stale/unbounded behavior.
