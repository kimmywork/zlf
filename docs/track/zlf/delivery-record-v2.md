---
status: completed
owner: kimmy
updated: 2026-07-08
source_requirements: docs/track/zlf/requirements-v1.md
---

# Delivery Record v2: Prolog + Indexed Query Integration Hardening

## Delivered

- Inspected current repository state and `stash@{0}` WIP.
  - Incorporated the valid WIP idea: all-edge enumeration in storage.
  - Did not apply the invalid WIP fragment that referenced a non-existent `self.facts` field.
- Extended Prolog parsing:
  - Multi-goal query parsing: `?a(X), b(X).`
  - Anonymous variable `_`.
  - Inequality operator `\=`.
- Extended Prolog execution over database facts:
  - `node/3`, `edge/4` with constant and variable matching.
  - Edge-type dynamic predicates: `works_at(alice, C)` maps to stored `works_at` edges.
  - Property predicates: `prop/3`, `node_property/3`, `has_property/3`.
  - Indexed predicates in Prolog conjunctions: `search`, `similar_to`, `after`, `before`, `time_range`/`between`.
- Improved import/export:
  - JSON import preserves explicit node and edge IDs.
  - JSON export returns actual stored nodes and edges.
- Improved stdio embedding support:
  - `index_embedding` now accepts precomputed embedding vectors as well as provider-generated text embeddings.
- Added tests for:
  - Database-backed rule inference with `\=`.
  - Graph + property composition.
  - Graph + BM25 composition.
  - Graph + vector similarity composition.
  - JSON-over-stdio import/export ID preservation.
  - JSON-over-stdio embedding + graph composite query.

## Verification Evidence

Command run:

```bash
cargo test
```

Result: passed.

Observed totals from the run:

- zlf-api unit: 3 passed
- zlf-api integration: 2 passed
- zlf-cli integration: 14 passed
- zlf-core: 17 passed
- zlf-embed: 1 passed
- zlf-index: 21 passed
- zlf-prolog: 40 passed
- zlf-query: 19 passed
- zlf-storage: 16 passed
- doc tests: all passed

## Remaining Gaps

- Full ISO Prolog support remains incomplete: no cut semantics in the main database-backed engine, no negation-as-failure, no arithmetic comparisons beyond `\=`, no persistent rule catalog, no standardization-apart for complex recursive rules.
- HTTP API exists as request/response JSON on `/api`; true HTTP JSON streaming/NDJSON is not implemented yet.
- Vector search is still brute-force over RocksDB entries, not ANN.
- Import pipeline accepts structured JSON; document/Markdown/LLM-assisted external knowledge-base parsing is not implemented.
- Query optimization is still execution-order driven; there is no cost-based planner.
