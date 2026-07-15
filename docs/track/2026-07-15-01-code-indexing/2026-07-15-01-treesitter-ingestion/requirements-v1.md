---
status: pending
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-15-01-code-indexing
version: 1
---

# Tree-sitter ingestion stage

Deliver a versioned parser registry, safe repository/file discovery, language detection, stable file identities, source fingerprints, and incremental extraction of syntax-backed symbols and source ranges for Java, C, C++, Python, Rust, JavaScript, TypeScript, Kotlin, Go, and Swift.

## Semantic enrichment policy

Tree-sitter is the always-available syntax baseline. Versioned optional enrichers may consume build-system metadata, dependency graphs, compiler/language tooling, and LSP results. Each enricher declares capabilities and records tool/version/configuration provenance. Missing or failed enrichers degrade to syntax-only indexing without labeling the semantic generation complete.

## Acceptance

- Java, C, C++, Python, Rust, JavaScript, TypeScript, Kotlin, Go, and Swift have pinned grammars, versioned adapters, and golden parse fixtures.
- Build/LSP enrichers are optional, bounded, provenance-bearing, and have syntax-only fallback tests.
- Ignore, symlink, binary, generated/vendor, encoding, and file-size policies are explicit and bounded.
- Repeated imports are idempotent; changed/deleted files converge without stale extracted records.
- Parse errors retain bounded diagnostics and do not publish partial file generations as complete.

## Non-goals

Retrieval ranking or claiming compiler-equivalent semantics when the relevant semantic enricher is absent.
