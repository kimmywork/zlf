---
status: pending
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-15-01-code-indexing
version: 1
---

# Code retrieval stage

Index symbol names, qualified names, signatures, documentation, and bounded code snippets with BM25 and compose lexical candidates with code graph/Prolog filters and expansion.

## Acceptance

- APIs support explicit repository/language/path/kind filters and finite top-k/candidate/depth budgets.
- Exact symbol lookup, BM25 ranking, caller/callee/import/containment expansion, and explanation provenance are tested.
- Default operation requires no embedding model or vector index.
- Disabled vector requests return the shared typed index-unavailable error rather than changing retrieval mode silently.

## Non-goals

Mandatory semantic embeddings, LSP transport, or unbounded whole-repository context assembly.
