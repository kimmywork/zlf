# Design review: optional vector embedding/index strategy v1

## Scope reviewed

User direction, parent requirements/change notes, Stage 03 exact lifecycle, Stage 06 HNSW evidence, disabled/default behavior, exact fallback, asynchronous rebuild, and the independent code-indexing boundary.

## Findings

### Critical/high

None.

### Medium — stale ANN must never be presented as current

An old ready ANN can remain queryable during rebuild only if its source identity still matches the requested published vector watermark. After exact mutation publication, routing must mark ANN stale and use exact until the replacement publication is ready; it cannot keep serving old ANN merely for availability.

**Resolution:** design updated contract is interpreted as “old snapshot only while identity-current”; mutation marks stale before scheduling. Tests must prove no deleted/updated stale hit.

### Medium — filtered ANN exhaustion requires correctness fallback

Bounded HNSW overfetch may return fewer than top-k eligible hits even when more exist. Returning that result as exact would violate current retrieval contracts.

**Resolution:** route filtered/exhausted cases to exact unless the ANN result is explicitly marked approximate. First facade integration uses exact fallback to preserve existing semantics.

### Medium — background worker lifetime and coalescing

Detached workers must not borrow `ZlfDatabase`; state and stores need owned `Arc`s. Concurrent requests must not spawn unbounded rebuilds.

**Resolution:** one shared rebuild controller owns `Arc` stores/state, allows one builder, and coalesces one pending rerun.

### Low — default-disabled test migration

Many existing vector tests call `ZlfDatabase::open`. They must opt into exact mode explicitly; non-vector tests should remain on the new disabled default to protect the contract.

## Decision

**Passed.** No critical or major findings remain. Implement in bounded slices and preserve exact as authoritative fallback.
