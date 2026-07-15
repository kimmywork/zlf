---
status: pending
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-15-01-code-indexing
version: 1
---

# Tree-sitter ingestion stage

Deliver zero-configuration scan-root bootstrap, a versioned parser registry, safe repository/worktree/file discovery, language detection, stable file identities, source fingerprints, and incremental extraction of syntax-backed symbols and source ranges for Java, C, C++, Python, Rust, JavaScript, TypeScript, Kotlin, Go, and Swift.

## Source blob persistence

Indexed UTF-8 source bytes are published to a dedicated content-addressed compressed blob store. File nodes reference blob identity and fingerprint; symbol ranges, BM25 documents, snippets, CFG, and visualization consume the same immutable bytes. Source-inclusive export is explicit, logs/metrics/errors omit source, and garbage collection respects all active/retained generation references. Binary/generated/vendor/oversized policies prevent unnecessary retention.

## Semantic enrichment policy

Tree-sitter is the always-available syntax baseline. Versioned optional enrichers may consume build-system metadata, dependency graphs, compiler/language tooling, and LSP results. Each enricher declares capabilities and records tool/version/configuration provenance. Missing or failed enrichers degrade to syntax-only indexing without labeling the semantic generation complete.

## Repository identity

First discovery generates a durable internal `RepositoryId` without modifying the repository. Git remotes, common-dir/history signals, and prior scan paths are rediscovery aliases. Unique path/remote moves preserve identity; ambiguous copies/forks create a new identity and warning.

## Acceptance

- Bootstrap requires no manifest and produces an inspectable discovery inventory.
- Repository identity survives uniquely recognized path/remote changes and distinguishes ambiguous forks/copies.
- Nested `.gitignore` and standard Git exclude behavior match independent Git fixtures across worktrees, nested repositories, and submodules.
- Java, C, C++, Python, Rust, JavaScript, TypeScript, Kotlin, Go, and Swift have pinned grammars, versioned adapters, and golden parse fixtures.
- Build/LSP enrichers are optional, bounded, provenance-bearing, and have syntax-only fallback tests.
- Ignore, symlink, binary, generated/vendor, encoding, and file-size policies are explicit and bounded.
- Repeated imports are idempotent; changed/deleted files converge without stale extracted records.
- Blob dedupe, reopen checksum validation, crash-safe publication, explicit source export, and generation-safe garbage collection are tested.
- Parse errors retain bounded diagnostics and do not publish partial file generations as complete.

## Non-goals

Retrieval ranking or claiming compiler-equivalent semantics when the relevant semantic enricher is absent.
