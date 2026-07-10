---
status: proposed
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-10
version: 1
---

# Stage 05 Requirements: Hybrid Retrieval and Prolog Composition

## Goal

Combine lexical, semantic, temporal, graph, property, and rule constraints in one bounded, explainable query path suitable for enterprise and agent retrieval.

## Requirements

- Preserve existing `bm25/3`, `vector_similar/3`, `temporal_on/2`, and `temporal_between/3` behavior or version/migrate intentional semantic changes.
- Add an option-bearing facade/predicate contract for top-k, threshold, field/chunk, model/analyzer generation, temporal constraints, and source exclusion.
- Support lexical-first, vector-first, and bound-entity access paths; planner output shows selected retrieval/index/filter strategy.
- Fuse lexical/vector ranks with an explicit method. Reciprocal-rank fusion is the recommended baseline because raw score scales differ.
- Support graph/label/property/rule filters and temporal validity constraints without fetching an unbounded global candidate relation.
- Return stable entity/document/chunk identity, per-retriever rank/score, fused score/rank, index generation, and optional explanation/provenance.
- Remote embedding generation occurs before WAM execution; the WAM receives a query vector/handle or uses a previously indexed source.
- External answer production is cursor/page based or otherwise bounded and preserves backtracking, cut, proof, and deterministic ordering.
- Define whether retrieval predicates are tableable and how index generation/mutation participates in table dependency identity.

## Verification

- Facade and WAM tests cover each retriever alone and joins in multiple goal orders.
- Tests cover bound candidates, empty indexes, incompatible generations/models, top-k, ties, cut, backtracking, proof leaves, restart, and index mutation.
- Hybrid quality reports compare lexical, vector, and fusion on the same judgments; fusion must not be declared better without measured metrics.
- Query reports include planning, candidate counts, filter selectivity, p50/p95/p99, peak materialized answers, and quality.

## Non-goals

- A second SQL/search query language or replacement for WAM planning.
- Directly adding BM25 and cosine scores.
- Unbounded global ranking inside a Prolog rule body.
