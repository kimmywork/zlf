# Code indexing requirements discovery v1

**Date:** 2026-07-15  
**Status:** draft, awaiting user decisions

## User-stated needs

1. Symbol-aware BM25 retrieval for CamelCase, snake_case, kebab-case, and long concatenated identifiers. The requirement is identifier-boundary subtokenization, not full character ngram, arbitrary suffix matching, typo tolerance, or fuzzy search.
2. Cross-module, cross-repository, and potentially cross-language symbol relationships, including server/client relationships.
3. Target scale: approximately 100,000 files, 1,000,000 symbols, and 3,000,000 symbol relationships.
4. A canonical identity decision for same-simple-name definitions in different packages.
5. Deep recursive caller/callee and bounded call-chain retrieval through a dedicated DSL.
6. Mermaid/PlantUML output for call chains, sequence views, code/file flowcharts, and class relationships.
7. Lower lexical/evidence weight for comments; executable code is source of truth.

## Confirmed current-system constraints

- Current graph algorithms are storage-backed BFS with `MAX_VISITED=100_000`, `MAX_RESULTS=10_000`, and default depth 32 (`crates/zlf-prolog/src/wam/providers/graph_algorithm.rs`). This cannot be treated as evidence for the requested million-symbol workload.
- Current BM25 analysis is pinned to `unicode_jieba_v1`; no code-symbol analyzer/ngram contract exists (`crates/zlf-index/src/profile.rs`).
- Project architecture requires one WAM runtime and forbids a second general Prolog evaluator (`AGENTS.md`). A code-query DSL should therefore compile to a bounded query-plan/graph-traversal IR and expose results to WAM rather than create another logic runtime.
- Existing graph storage already supports distinct node IDs, so duplicate simple symbol names are structurally possible. The unresolved question is canonical identity and resolution semantics, not whether RocksDB can store multiple nodes.

## Confirmed symbol analyzer semantics

Confirmed by the user on 2026-07-15: the intended behavior is identifier-boundary subtokenization, not full character ngram.

```text
ServiceDispatcher  -> service, dispatcher
service_dispatcher -> service, dispatcher
service-dispatcher -> service, dispatcher
```

A query for `Dispatcher` must retrieve `ServiceDispatcher`. Arbitrary suffix/middle-fragment matching and misspelled queries are not required. The index should retain the normalized complete identifier plus boundary-derived subtokens; adjacent subtoken shingles may improve ranking but must not change the matching contract.

## Confirmed language scope

Confirmed by the user on 2026-07-15. The initial code-indexing language set is:

- Java
- C
- C++
- Python
- Rust
- JavaScript
- TypeScript
- Kotlin
- Go
- Swift

Each language uses an explicitly pinned Tree-sitter grammar and a versioned language adapter. JavaScript and TypeScript, and C and C++, remain distinct adapters where their syntax and symbol rules differ.

## Confirmed semantic enrichment policy

Confirmed by the user on 2026-07-15: build-system metadata, dependency metadata, compiler/language tooling, and Language Server Protocol implementations may be used as additional semantic evidence.

The baseline remains Tree-sitter syntax extraction. Semantic enrichers are optional and capability-declared; indexing must still produce bounded `declared`/`inferred`/`unresolved` results when a toolchain is unavailable. Every enriched symbol or edge records the provider/tool, version, repository revision, configuration/classpath/compile database identity, certainty, and source provenance. External tools must not silently replace syntax facts, and failures must degrade to the baseline rather than publish a partially complete semantic generation as complete.

Candidate evidence sources include Cargo metadata/rust-analyzer, Gradle/Maven/JDT/Kotlin tooling, `compile_commands.json`/clangd/libclang, `go list`/gopls, `package.json`/`tsconfig`/TypeScript language service, SwiftPM/SourceKit, and Python project/import metadata or Pyright.

## Recommended requirement additions

- Separate syntax extraction from semantic resolution. Tree-sitter provides syntax; language adapters, compiler metadata, build manifests, IDLs, OpenAPI/gRPC/protobuf schemas, and explicit mappings may provide stronger cross-repo/cross-language evidence.
- Persist relation provenance and certainty (`resolved`, `declared`, `inferred`, `unresolved`) independently from lexical field weight.
- Model repository identity, revision/commit, module/package, enclosing type, symbol kind, signature/overload, source range, extractor version, and source fingerprint.
- Preserve unresolved external references and reconcile them when another repository is indexed.
- Require typed/directional adjacency indexes, repository/language/kind filters, pagination, max depth, max visited, max paths, timeout, cycle policy, and deterministic ordering.
- Treat “all callers” (reachable symbol set) separately from “all call paths” (potentially exponential). Path queries must always be bounded.
- Produce a language-neutral visualization IR first, then Mermaid and PlantUML renderers. Static call order is not runtime sequence; sequence diagrams must be labeled static/approximate unless trace data is available.
- Treat function flowcharts as language-specific approximate CFG extraction; Tree-sitter alone is not compiler-equivalent.
- Add incremental file ownership, delete convergence, repository ACL/visibility, secret-safe snippets, schema/extractor versioning, and rebuild equivalence.
- Add target-scale benchmark tiers culminating in 100K files / 1M symbols / 3M relationships, with mutation, reopen, traversal, RSS, disk, and stale-edge evidence.

## Confirmed identity model

Confirmed by the user on 2026-07-15: use one node per concrete symbol definition, never one node per simple name.

```text
SymbolDefinitionId =
  repository identity
  + language
  + module/package/namespace
  + enclosing symbol path
  + symbol kind
  + normalized signature/overload discriminator
```

Keep `simple_name` and `qualified_name` as indexed attributes. The repository revision, source fingerprint, and index generation version the node; they should not normally create a new logical symbol ID. If historical snapshots must coexist, add snapshot-qualified `SymbolVersion` records rather than changing the current-definition identity. Optionally add separate logical/external contract nodes for identities shared across implementations, such as an HTTP route, protobuf RPC, OpenAPI operation, JVM symbol, Cargo item, or manually declared service contract. Definition nodes connect to those contract nodes with provenance-bearing edges.

This permits multiple `ServiceDispatcher` definitions while still supporting simple-name retrieval and cross-language server/client linkage.

## Open decision queue

1. **Confirmed:** concrete definitions are separate nodes; simple names never merge definitions.
2. **Confirmed:** split identifier boundaries and index full normalized identifiers plus subtokens; no full character ngram, arbitrary suffix matching, or typo tolerance.
3. **Confirmed:** Java, C, C++, Python, Rust, JavaScript, TypeScript, Kotlin, Go, and Swift are in the initial language scope.
4. **Confirmed:** build systems, dependency metadata, compiler/language tooling, and LSP implementations may optionally enrich Tree-sitter syntax facts.
5. Define cross-repo resolution scope and manual mapping/contract ingestion.
6. Define DSL path semantics: reachable sets, simple paths, shortest/top-N paths, cycles, ranking, and required bounds.
7. Define static versus runtime-backed visualization claims.
8. Define repository tenancy/ACL and revision retention.
