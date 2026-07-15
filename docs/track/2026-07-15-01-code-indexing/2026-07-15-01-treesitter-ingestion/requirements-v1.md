---
status: pending
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-15-01-code-indexing
version: 1
---

# Tree-sitter ingestion stage

Deliver a versioned parser registry, safe repository/file discovery, language detection, stable file identities, source fingerprints, and incremental extraction of syntax-backed symbols and source ranges.

## Acceptance

- At least one statically typed and one dynamic language have pinned grammars and golden parse fixtures.
- Ignore, symlink, binary, generated/vendor, encoding, and file-size policies are explicit and bounded.
- Repeated imports are idempotent; changed/deleted files converge without stale extracted records.
- Parse errors retain bounded diagnostics and do not publish partial file generations as complete.

## Non-goals

Code relationship resolution beyond syntax-local containment/import facts, retrieval ranking, or compiler-equivalent semantics.
