# Stage 03 Implementation Progress v5

## Increment V3 — ANN evidence gate

**Status:** completed as an approved defer decision on 2026-07-11.

`hnsw_rs` 0.3.4 has persistence support, but canonical `usize` ID mapping, random alternate dump basenames under mmap, and lack of public delete semantics require a nontrivial immutable-generation/tombstone design. Per the confirmed optional-ANN and function-first policies, no speculative ANN runtime path was added.

Exact RocksDB search remains the production backend and correctness fallback. See parent `change-note-v4-defer-vector-ann.md` for evidence and re-entry criteria.

### Next

- Remove the node-only prototype `VectorIndex` write/read path in favor of exact document/model/generation APIs.
- Add exact 1K/10K retrieval/build/update/RSS/disk evidence and deterministic provider throughput evidence.
- Complete Stage 03 cumulative review and delivery acceptance.
