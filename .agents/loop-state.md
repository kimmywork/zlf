# Loop State: zlf

**Updated:** 2026-07-15
**Autonomy:** full
**Current phase:** solution design ready

## Active Track

`docs/track/2026-07-15-01-code-indexing/`

Goal: build a zero-configuration, Tree-sitter-backed enterprise code knowledge index with canonical code graph facts, symbol BM25, cross-repository/cross-language relationships, bounded zlf-Prolog traversal predicates, and static Mermaid/PlantUML visualization.

Requirements discovery is complete and confirmed. Do not ask further discovery questions unless implementation/design uncovers a true contradiction or blocker.

Primary source:

- `docs/track/2026-07-15-01-code-indexing/research/requirements-discovery-v1.md`

## Delivery Topology

| Stage | Status | Responsibility |
|---|---|---|
| `2026-07-15-01-treesitter-ingestion` | pending | zero-config Git scan, pinned grammars/adapters, source blobs, incremental file lifecycle, optional semantic enrichers |
| `2026-07-15-02-code-graph` | pending | canonical repositories/files/symbols/contracts, occurrence edges, provenance/certainty, deduplicated adjacency |
| `2026-07-15-03-code-retrieval` | pending | symbol BM25, specialized zlf-Prolog predicates, bounded paths/cycles, static visualization IR/renderers |
| `2026-07-15-04-code-benchmark` | pending | correctness, mutation/reopen, and 100K-file/1M-symbol/3M-edge scale evidence |

## Confirmed Requirements

### Bootstrap and languages

- Zero-configuration scan-root bootstrap; no required manifest.
- Follow nested `.gitignore` and standard Git excludes.
- Automatically discover repositories/worktrees, nested repos/submodules, languages, modules/packages, build systems, dependencies, and contracts.
- Initial languages: Java, C, C++, Python, Rust, JavaScript, TypeScript, Kotlin, Go, and Swift.
- Pin every Tree-sitter grammar and version every language adapter.
- Tree-sitter is the syntax baseline; optional build/compiler/LSP enrichers are allowed and provenance-bearing.

### Identity and lifecycle

- Generate a durable internal `RepositoryId`; Git path/remote/history signals are rediscovery aliases, not canonical identity.
- Each concrete symbol definition is a separate node; identical simple names never merge definitions.
- Symbol ID includes repository, language, module/package/namespace, enclosing path, kind, and normalized signature/overload discriminator.
- One active revision per repository. Revision changes incrementally replace active file-owned nodes/edges.
- Historical commits are not queryable by default; future history may use explicit immutable snapshots.
- Indexed UTF-8 source is stored in a compressed content-addressed blob store for reproducible parsing, snippets, CFG, and visualization.

### Canonical graph

- Repositories, files, modules/packages, symbols, contracts, and unresolved external symbols are ordinary canonical graph nodes.
- Contains/defines/calls/references/imports/extends/implements/instantiates/decorates and contract links are ordinary typed edges.
- Canonical relationship evidence is occurrence-level and retains source range, metadata, provider, provenance, and certainty.
- Traversal uses a rebuildable deduplicated `(source, kind, target)` incoming/outgoing adjacency projection with occurrence summary.
- Certainty classes: `resolved`, `specified`, `declared`, `inferred`, `unresolved`.
- Cross-repo/language links use first-class contract/external-symbol nodes plus automatic evidence and optional versioned mapping manifests.

### Search and query

- BM25 is symbol-only: complete/simple/qualified names, identifier-boundary subtokens, kind, signature/type metadata, and lower-weight doc/comment metadata.
- Split CamelCase, acronym transitions, snake_case, kebab-case, and letter/digit boundaries.
- `Dispatcher` must find `ServiceDispatcher`; no full character ngram, arbitrary suffix/middle matching, typo tolerance, or fuzzy search.
- General raw-source/file search is out of scope; use ripgrep.
- zlf-Prolog remains the only textual DSL.
- Ordinary node/edge/property queries handle direct facts and simple joins.
- Specialized `code_search`, `code_callers`, `code_callees`, `code_path`, `code_cycle`, and rendering predicates compile to a shared typed bounded `CodeQuery` AST/executor also used by JSON/HTTP.
- Reachable symbol sets and concrete paths are distinct operations.
- Paths are deterministic Top-N shortest simple paths; cycles/SCCs are separate.
- Every traversal has finite depth, visited-symbol, traversed-edge, path-count, result, and timeout budgets and reports exhaustion/truncation.
- Keep one WAM runtime; do not introduce a second evaluator.

### Visualization

- Static analysis only.
- One bounded visualization IR feeds Mermaid and PlantUML renderers.
- Initial views: call graph, class/type relations, static sequence diagrams, and language-adapter CFG flowcharts.
- Sequence/CFG output is labeled static/approximate and retains symbol/source/provenance/certainty metadata.

### Scale target

- 100,000 code files.
- 1,000,000+ symbols.
- 3,000,000+ occurrence relationships.
- Validation must cover initial build, incremental update/delete, reopen, rebuild equivalence, stale-result absence, traversal quality/latency, RSS, and disk.

## Deferred / Non-goals

- Repository/user ACL and permission-aware traversal.
- Runtime trace ingestion and observed sequence diagrams.
- All-commit history by default.
- General raw-source full-text search.
- Mandatory vector embedding for code indexing.
- Compiler-equivalent precision when semantic enrichers are absent.
- A second Prolog/query runtime.

## Relevant Delivered Baseline

- Optional vector strategy is delivered: embedding disabled by default; exact/HNSW selectable; asynchronous immutable HNSW rebuild with exact fallback (`d668cb9`).
- Current CLI/REPL/IndexProfile usage guide is refreshed and TypeScript SDK material removed (`380cf9f`).
- Hybrid retrieval/index lifecycle work remains available as infrastructure, but code indexing is now the active design track.

## Next Action

Produce `solution-design-v1.md` for the code-indexing parent and stages, including:

1. canonical schema and stable identity encoding;
2. zero-config Git scanner and source blob lifecycle;
3. grammar/adapter and semantic-enricher interfaces;
4. occurrence-edge write path and deduplicated adjacency layout;
5. symbol analyzer/Tantivy field design;
6. `CodeQuery` AST, zlf-Prolog predicates, planner, paging, proof, tabling, and invalidation;
7. visualization IR/renderers;
8. phased implementation plan and 1K/10K/100K-to-1M/3M verification matrix.

Run independent design review before implementation. No production code changes before the design gate passes.
