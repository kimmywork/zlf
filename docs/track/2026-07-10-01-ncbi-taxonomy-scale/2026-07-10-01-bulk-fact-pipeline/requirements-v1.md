---
status: done
scope_type: stage
created: 2026-07-10
parent_id: 2026-07-10-01-ncbi-taxonomy-scale
version: 1
---

# Bulk Fact Pipeline Stage

Deliver a restricted streaming ground-fact compiler and WriteBatch loader whose node/edge/index semantics are shared with normal fact writes. Packs must be versioned, checksummed, bounded-memory, idempotent, and query-ready. Acceptance is semantic parity on fixtures, rejection of invalid packs/facts, prefix-bounded index access, and focused storage/query tests.
