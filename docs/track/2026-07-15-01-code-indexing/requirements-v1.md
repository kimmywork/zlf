---
status: pending
scope_type: parent
created: 2026-07-15
version: 1
---

# Code repository indexing and query requirements

## Elevator pitch

Integrate Tree-sitter so zlf can turn repositories into queryable code symbols, lexical documents, and graph relationships. The default code-index path emphasizes BM25 plus structural graph traversal and does not require vector embedding.

## Primary users

- Repository maintainers tracing definitions, references, imports, callers, and dependencies.
- Coding agents retrieving exact symbols and structurally related code under bounded context budgets.
- Operators importing large symbol-heavy repositories without remote embedding cost.

## Product scenarios

1. Parse a supported repository incrementally and persist files, symbols, source ranges, and relationships with stable identities.
2. Search exact/lexical symbol names, signatures, documentation, and code snippets with BM25.
3. Traverse containment, import, definition/reference, call, inheritance/implementation, and dependency edges through existing graph/Prolog queries.
4. Combine lexical candidates with graph filters/expansion and return source paths/ranges suitable for downstream tools.
5. Re-index changed/deleted files without leaving stale symbols or edges.

## Scope map

| Stage | Summary | Status |
|---|---|---|
| `2026-07-15-01-treesitter-ingestion` | Parser registry, file discovery, stable syntax/symbol extraction, incremental lifecycle | pending |
| `2026-07-15-02-code-graph` | Canonical code entity/edge schema and language-neutral relationship lowering | pending |
| `2026-07-15-03-code-retrieval` | BM25, graph/Prolog composition, bounded code query APIs and explanations | pending |
| `2026-07-15-04-code-benchmark` | Public/generated repositories, correctness oracles, scale and mutation evidence | pending |

## Parent requirements

- Initial language scope is Java, C, C++, Python, Rust, JavaScript, TypeScript, Kotlin, Go, and Swift.
- Tree-sitter grammars are explicit, versioned dependencies; unsupported languages fail clearly.
- Repository path, file identity, language, symbol kind/name, byte/line ranges, signatures, and source fingerprints are canonical and durable.
- Parsing is bounded and excludes generated/vendor/binary/oversized content through explicit policy.
- Graph storage remains source of truth; BM25 is a derivative index.
- Embedding is disabled by default and is not required for any code-index acceptance criterion.
- Queries expose explicit top-k/candidate/expansion/depth limits and deterministic ordering.
- Incremental update/delete and full rebuild converge to the same logical index.
- Existing WAM, `FactProvider`, storage, BM25, profile, generation, and coordinator paths are reused; no second query runtime is introduced.

## Non-goals

- Full compiler/type-checker equivalence across languages.
- LSP server implementation in the first delivery.
- Semantic vector search as a required code retrieval path.
- Executing untrusted repository code or build scripts.
- Whole-program dynamic call-graph precision.

## Parent acceptance

- Java, C, C++, Python, Rust, JavaScript, TypeScript, Kotlin, Go, and Swift are supported end-to-end at the syntax/symbol contract level; any language-specific semantic-resolution limitations are explicit.
- Definition/reference/import/call fixtures match independent source-range oracles.
- Name/signature/doc/code BM25 plus graph composition answers bounded repository questions.
- Changed/deleted files leave no stale symbols, documents, or relationships after convergence and reopen.
- A reproducible repository benchmark reports parse/index/query time, correctness, RSS, and disk.

## Risks

Grammar/version proliferation, ambiguous references without a compiler, generated-source scale, symlinks/path traversal, and language-specific semantics. Stages must preserve raw syntax provenance and label inferred edges rather than claiming compiler certainty.
