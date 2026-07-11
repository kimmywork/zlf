---
status: in_progress
scope_type: parent
created: 2026-07-11
version: 1
---

# Change Note v1: Function-First Backend Policy

## Reason

Product direction on 2026-07-11: prefer simple, mainstream, stable implementations that establish complete functionality. Defer comparative backend selection and specialized optimization until the functional path is stable.

## Changes

- BM25: select Tantivy as the initial production backend; retain a versioned zlf analyzer adapter and independent correctness fixtures, but remove the custom-RocksDB comparison gate.
- Vector: implement canonical exact search first, then use `hnsw_rs` as the initial embedded ANN derivative if its current persistence API integrates cleanly. ANN does not block functional delivery; exact remains fallback.
- Temporal: implement ordered RocksDB event/start/end/open-end indexes; defer buckets or specialized interval structures.
- Hybrid Prolog: use explicitly bounded materialization/paging first. A WAM-owned provider cursor is an optimization and is deferred unless bounded materialization cannot meet functional limits.
- Benchmarks remain verification tools, but no comparative technology-selection phase or performance optimization blocks the first functional delivery.

## Unchanged

Correctness oracles, lifecycle safety, generation isolation, explicit limits, deterministic ordering, no remote embedding in WAM, 100K maximum local scope, and workspace quality gates remain required.

## Rollback

Tantivy, ANN, temporal acceleration, and provider paging remain behind common contracts/generation boundaries, so later evidence can replace or optimize them without changing canonical graph or document identity.
