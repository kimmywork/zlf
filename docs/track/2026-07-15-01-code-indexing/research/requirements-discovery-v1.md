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

## Confirmed cross-repository contract model

Confirmed by the user on 2026-07-15: cross-repository and cross-language resolution uses first-class contract/external-symbol nodes, automatic evidence, and optional versioned mapping manifests.

Contract identities may represent OpenAPI operations, HTTP method/normalized routes, protobuf package/service/RPC, GraphQL fields, message topics, package/artifact coordinates, exported external symbols, or shared headers. Client and implementation symbols connect to the contract node rather than being merged or linked by an unqualified-name guess. Resolution precedence is compiler/LSP evidence, shared schema/build dependency, explicit mapping, then syntax/name/string heuristics. Explicit operator mappings are `specified`, not compiler-`resolved`, and retain manifest/version provenance.

## Confirmed traversal semantics

Confirmed by the user on 2026-07-15: reachable symbol sets and concrete call paths are distinct operations. Caller/callee reachability returns deduplicated symbols. Path retrieval returns bounded Top-N shortest simple paths; a path does not repeat nodes, cycles/SCCs are represented separately, and every traversal has finite depth, visited-symbol, traversed-edge, path-count, and timeout budgets. Results report exhaustion/truncation. Contract edges participate only when explicitly requested.

## Confirmed zlf-Prolog query surface

Confirmed by the user on 2026-07-15: zlf-Prolog remains the only textual query DSL. Concrete repositories, files, symbols, contracts, and source-derived relationships are persisted as ordinary canonical graph nodes/edges and are directly queryable through normal Prolog label, property, edge-type shortcut, rule, and join syntax.

Specialized predicates are derived execution paths, not a second storage model or language. Ranked symbol search and expensive transitive/path/cycle/visualization operations such as `code_callers`, `code_callees`, `code_path`, and `code_cycle` compile into a typed bounded `CodeQuery` AST and specialized adjacency/traversal executor. JSON/HTTP APIs reuse the same AST rather than defining different semantics.

The WAM composes these bounded derived results with ordinary facts, graph predicates, properties, rules, proof, and tabling; it does not execute million-symbol traversal through ordinary Prolog recursion, and no second general evaluator is introduced. Deep traversal requires bound source/target modes and finite options. Results carry graph generation, watermark, provenance, and explicit exhaustion/truncation metadata.

## Confirmed static visualization scope

Confirmed by the user on 2026-07-15: visualization is static-analysis based. A language-neutral bounded visualization IR is the source for Mermaid and PlantUML renderers. Initial views include call graphs, class/type relationships, static sequence diagrams along selected call paths, and language-adapter CFG flowcharts. Sequence and CFG views are explicitly labeled static/approximate; runtime trace ingestion and observed sequence semantics are not required.

Visualization retains symbol IDs, repository/path/source ranges, edge certainty, and provenance. Requests have finite node, edge, depth, path, and timeout budgets and report truncation. Renderers do not own independent query semantics.

## Confirmed active-revision model

Confirmed by the user on 2026-07-15: each repository has one active indexed revision. Switching revision incrementally replaces the active file/symbol/relation graph. Stable logical symbol IDs do not include commit identity; revision, source fingerprint, and extractor generation version the current definition. Historical commits are not all queryable by default. Future history support may publish explicitly selected immutable snapshots such as release tags or audit baselines.

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
5. **Confirmed:** use first-class contract/external-symbol nodes, automatic evidence, and optional versioned mapping manifests for cross-repo/cross-language linkage.
6. **Confirmed:** separate reachable sets from paths; paths are bounded Top-N shortest simple paths, cycles are separate, and exhaustion is explicit.
7. **Confirmed:** zlf-Prolog is the only textual DSL; dedicated code predicates compile to a shared bounded `CodeQuery` AST/executor also used by JSON/HTTP.
8. **Confirmed:** initial visualization is static; bounded visualization IR feeds Mermaid/PlantUML, with sequence/CFG views labeled approximate.
9. **Confirmed:** one active revision per repository; historical revisions require future explicit immutable snapshots.
10. Define repository tenancy/ACL.
