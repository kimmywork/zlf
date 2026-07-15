---
status: pending
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-15-01-code-indexing
version: 1
---

# Code retrieval stage

Index symbol names, qualified names, signatures, documentation, and bounded code snippets with BM25 and compose lexical candidates with code graph/Prolog filters and expansion.

## Confirmed symbol analysis

The code-symbol analyzer stores the normalized complete identifier and identifier-boundary subtokens. It splits CamelCase, acronym transitions, snake_case, kebab-case, and letter/digit boundaries. For example, `ServiceDispatcher` indexes `service` and `dispatcher`, so `Dispatcher` retrieves it. Full character ngram, arbitrary middle/suffix fragments, typo tolerance, and fuzzy matching are not required.

## Confirmed traversal semantics

Reachable caller/callee symbol sets and concrete call paths are separate operations. Reachability deduplicates symbols. Path retrieval returns bounded Top-N shortest simple paths, with cycles/SCCs represented separately. Every request has finite depth, visited-symbol, traversed-edge, path-count, and timeout budgets; budget exhaustion is explicit. Cross-repository contract edges are opt-in per query.

## Acceptance

- APIs support explicit repository/language/path/kind filters and finite top-k/candidate/depth budgets.
- Caller/callee sets, shortest simple paths, Top-N deterministic path ordering, cycle reporting, contract-edge opt-in, and exhausted/truncated metadata are independently tested.
- Exact symbol lookup, boundary-subtoken symbol lookup, BM25 ranking, caller/callee/import/containment expansion, and explanation provenance are tested.
- Default operation requires no embedding model or vector index.
- Disabled vector requests return the shared typed index-unavailable error rather than changing retrieval mode silently.

## Non-goals

Mandatory semantic embeddings, LSP transport, or unbounded whole-repository context assembly.
