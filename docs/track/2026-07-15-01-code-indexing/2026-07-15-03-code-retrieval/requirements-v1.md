---
status: pending
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-15-01-code-indexing
version: 1
---

# Code retrieval stage

Index symbol names, qualified names, kinds, signatures/type metadata, and lower-weight documentation metadata with BM25 and compose lexical candidates with code graph/Prolog filters and expansion. General file/raw-source full-text search is intentionally left to ripgrep; zlf does not index complete file bodies, arbitrary source tokens, or string literals for that purpose.

## Confirmed symbol analysis

The code-symbol analyzer stores the normalized complete identifier and identifier-boundary subtokens. It splits CamelCase, acronym transitions, snake_case, kebab-case, and letter/digit boundaries. For example, `ServiceDispatcher` indexes `service` and `dispatcher`, so `Dispatcher` retrieves it. Full character ngram, arbitrary middle/suffix fragments, typo tolerance, and fuzzy matching are not required.

## Confirmed traversal semantics

Reachable caller/callee symbol sets and concrete call paths are separate operations. Reachability deduplicates symbols. Path retrieval returns bounded Top-N shortest simple paths, with cycles/SCCs represented separately. Every request has finite depth, visited-symbol, traversed-edge, path-count, and timeout budgets; budget exhaustion is explicit. Cross-repository contract edges are opt-in per query.

## Confirmed query-language architecture

zlf-Prolog is the only textual DSL. Persisted repository/file/symbol/contract nodes and typed source relationships are ordinary graph facts, so direct lookups, one-hop edges, bounded joins, and application rules use normal label/property/edge shortcut predicates.

Dedicated `code_callers`, `code_callees`, `code_path`, and `code_cycle` predicates are derived optimized execution paths for transitive reachability, Top-N paths, cycles, ranking, and visualization. They compile into a typed bounded `CodeQuery` AST and specialized adjacency/traversal executor; they do not introduce separate facts or a second language. Ranked symbol search may also use a code-specific BM25 predicate over the same persisted symbols. JSON/HTTP reuse the same AST. The WAM composes bounded results with ordinary facts/rules/proof/tabling but does not perform million-symbol traversal through ordinary recursive Prolog evaluation. Bound modes, finite budgets, generation/watermark identity, provenance, and exhaustion metadata are mandatory.

## Confirmed visualization scope

Visualization is static-analysis based. A shared bounded visualization IR feeds Mermaid and PlantUML renderers for call graphs, class/type relationships, static sequence diagrams along selected paths, and language-adapter CFG flowcharts. Sequence and CFG outputs are labeled static/approximate. Every view preserves symbol/location and relation certainty/provenance, enforces finite node/edge/depth/path/timeout budgets, and reports truncation. Runtime trace enrichment is not required.

## Acceptance

- APIs support explicit repository/language/path/kind filters and finite top-k/candidate/depth budgets.
- Equivalent visualization IR produces deterministic Mermaid and PlantUML call/class/static-sequence/CFG fixtures with approximation and truncation metadata.
- Ordinary Prolog queries answer direct symbol/property/relationship joins from canonical graph facts.
- Specialized Prolog predicates and JSON requests compile to the same `CodeQuery` AST and produce equivalent ordered results and exhaustion metadata.
- Caller/callee sets, shortest simple paths, Top-N deterministic path ordering, cycle reporting, contract-edge opt-in, and exhausted/truncated metadata are independently tested.
- Exact symbol lookup, boundary-subtoken symbol lookup, symbol-metadata BM25 ranking, caller/callee/import/containment expansion, and explanation provenance are tested.
- `code_search` returns symbol hits only; no file-scope BM25 result or raw-source search claim is made.
- Default operation requires no embedding model or vector index.
- Disabled vector requests return the shared typed index-unavailable error rather than changing retrieval mode silently.

## Non-goals

Mandatory semantic embeddings, LSP transport, or unbounded whole-repository context assembly.
