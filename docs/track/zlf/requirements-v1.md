---
status: in_progress
owner: kimmy
updated: 2026-07-08
scope_type: enhancement
---

# Requirements v1: Prolog + Indexed Query Integration Hardening

## Goal

Close the current implementation gap between the existing zlf prototype and the desired database-backed logic engine:

1. Prolog queries should use RocksDB-backed graph facts directly instead of requiring all facts to be preloaded.
2. Queries should compose graph predicates, property predicates, BM25 search, vector similarity, and temporal predicates in one Prolog conjunction.
3. JSON-over-stdio and HTTP JSON APIs should be integration-testable for these combined query paths.
4. JSON import/export should preserve explicit node/edge IDs so an external knowledge base can be loaded and queried deterministically.
5. Fuzzy embedding + Prolog reasoning should be represented by a concrete built-in predicate path that can be extended later.

## Current Increment Scope

Because the full target is larger than one safe change, this increment focuses on verified foundations:

- Parse and execute multi-goal Prolog queries such as `?edge(knows, alice, X, P), prop(X, name, "Bob").`
- Treat edge types as dynamic Prolog predicates, e.g. `works_at(alice, C)` uses stored `works_at` edges.
- Support property predicates over stored node/edge properties via `prop/3`, `node_property/3`, and `has_property/3`.
- Support `search/2`, `search/3`, `similar_to/2`, `similar_to/3`, `after/2`, `before/2`, `time_range/3` in the Prolog engine so they can compose with graph predicates.
- Add all-edge enumeration and deterministic JSON import/export of explicit IDs.
- Add integration tests covering stdio and core query combinations.

## Acceptance Criteria

- AC-001: A rule like `colleague(X, Y) :- works_at(X, C), works_at(Y, C), X \= Y.` returns correct database-backed solutions.
- AC-002: A query can combine graph and property constraints in one Prolog conjunction.
- AC-003: A query can combine graph and BM25 search in one Prolog conjunction.
- AC-004: A query can combine graph and vector similarity in one Prolog conjunction when embeddings are indexed.
- AC-005: Temporal predicates can participate in Prolog conjunctions and bind node IDs.
- AC-006: JSON import preserves provided IDs and JSON export returns actual stored nodes and edges.
- AC-007: Existing `cargo test` remains passing.

## Out of Scope for This Increment

- Full ISO Prolog completeness.
- Cost-based query optimizer.
- Approximate nearest-neighbor vector index.
- Production external knowledge-base document parsing pipeline.
- Persistent rule catalog beyond the current process lifetime.
