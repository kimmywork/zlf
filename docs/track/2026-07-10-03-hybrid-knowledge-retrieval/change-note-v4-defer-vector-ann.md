# Change Note v4: Defer the first ANN derivative

**Date:** 2026-07-11  
**Status:** accepted under the confirmed optional-ANN policy

## Decision

Stage 03 ships the exact generation/model/document-scoped RocksDB backend as the first production vector backend. `hnsw_rs` integration is deferred and does not block Stage 03 functional delivery.

## Evidence

`hnsw_rs` 0.3.4 was inspected from its published crate source:

- `AnnT` identifies points only by process-sized `usize`, requiring an additional durable canonical-ID mapping.
- Its persistence API writes separate graph/data files and may choose a random alternate basename instead of replacing files already used by mmap.
- The public `AnnT` API provides insert/search/dump but no delete operation; correct update/delete behavior therefore requires generation rebuild/tombstone policy and additional lifecycle validation.

These are solvable, but not a straightforward wrapper around the now-correct exact store. Implementing them before vector/temporal/hybrid functional convergence would violate the accepted function-first direction.

## Consequences

- Exact cosine/dot retrieval remains the oracle, production backend, and fallback.
- `AnnBackend` remains a design contract, not speculative code or a runtime branch.
- Stage 06 benchmark evidence may reopen ANN selection. Any later implementation must provide canonical ID mapping, immutable generation dumps, corruption fallback, update/delete policy, fresh-process Recall@k, RSS, and disk evidence.
- No approximate quality or scale claim is made for Stage 03.
