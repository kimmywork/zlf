# Stage 02 Implementation Progress v2

## Increment B1 — Tantivy functional integration

**Status:** completed on 2026-07-11

### Delivered

- Replaced prototype RocksDB token-count accumulation with Tantivy's real corpus-normalized BM25 backend.
- Versioned Jieba analysis is applied consistently to indexed documents and term queries.
- Bounded explicit top-k search with stable ID tie-breaking.
- Exact document replacement removes obsolete terms before adding current content.
- Delete, batch update, document count, commit/reload, and fresh-process reopen behavior.
- Existing query/provider facade now receives real BM25 scores without changing WAM architecture.

### Verification

- Verification: repeated TF ranking and bounded top-k → pass.
- Verification: replacement removes obsolete postings and delete removes live result → pass.
- Verification: Chinese/English mixed search and fresh-process reopen → pass.
- Verification: equal scores tie-break by stable ID → pass.
- Verification: full `zlf-index` tests and focused clippy/format/size/diff gates → pass.

### Next

B2 adds canonical field/chunk document identity, field filters/weights, diagnostics, and explicit backend schema validation.
