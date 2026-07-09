# Research Notes: zlf Kernel Enhancements

This folder contains implementation-level research notes for the zlf kernel enhancement track. These notes are intentionally more concrete than requirements and plan documents: they define key layouts, data structures, predicate contracts, algorithm steps, and verification strategies.

## Documents

| Document | Purpose |
|---|---|
| `fact-storage-indexing.md` | Canonical fact identity, RocksDB key layout, graph indexes, mutation events, and deletion semantics. |
| `builtin-predicates-and-node-view.md` | Exact builtin/provider predicate contracts, node view shapes, rule/predicate introspection, and Prolog-facing semantics. |
| `graph-algorithms.md` | Storage-backed graph algorithm builtins such as neighbors, degree, reachable, and shortest path. |
| `tabling-and-incremental-tabling.md` | Deterministic tabling MVP, table store layout, dependency tracking, invalidation, lazy recompute, and later delta maintenance. |

## Implementation ordering

The implementation should use these documents in this order:

1. `fact-storage-indexing.md`
2. `builtin-predicates-and-node-view.md`
3. `graph-algorithms.md`
4. `tabling-and-incremental-tabling.md`

Reason: incremental tabling depends on stable fact identities, mutation events, and predicate/rule dependency metadata. Graph algorithms can deliver cycle-safe path queries before full tabling is ready and also provide test fixtures for tabling later.
