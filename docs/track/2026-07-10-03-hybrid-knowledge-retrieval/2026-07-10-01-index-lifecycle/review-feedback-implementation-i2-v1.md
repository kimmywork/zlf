# Review Feedback: Stage 01 Increment I2

## Scope

Cumulative self-review of Stage 01 requirements/design, I1 contracts, and I2 atomic mutation/outbox implementation. No independent reviewer subagent was available.

## Findings

### I2-R1
- **Origin phase:** implementation
- **Severity:** major
- **Type:** correctness
- **Description:** The first draft made `delete_node` publish only a node tombstone, allowing incident edges to remain.
- **Evidence:** Stage acceptance requires node deletion/cascade to produce exact entity projections.
- **Suggested fix:** make canonical node deletion collect incident edges and commit edge tombstones followed by the node tombstone in one batch.
- **Resolution:** fixed in `Storage::delete_node`; lifecycle cascade tests pass.

### I2-R2
- **Origin phase:** implementation
- **Severity:** major
- **Type:** correctness
- **Description:** Allocating multiple cascade events by repeatedly reading the persisted sequence would assign duplicate sequence values because earlier values existed only in the uncommitted batch.
- **Suggested fix:** allocate a checked in-memory contiguous range while holding the storage write mutex, then write the final sequence in the batch.
- **Resolution:** fixed in `commit_node_cascade_delete`; concurrent and cascade sequence tests pass.

### I2-R3
- **Origin phase:** implementation
- **Severity:** minor
- **Type:** scope
- **Description:** `write_record_plans` bulk loading does not emit per-entity events.
- **Evidence:** Stage design assigns bulk sessions and one rebuild marker to I4 rather than I2.
- **Resolution:** accepted dependency; not represented as a canonical mutation API. I4 remains required before Stage 01 acceptance.

## Decision

**Pass for increment I2.** No unresolved critical/major findings. I3 property mutation may proceed. Full Stage 01 delivery remains open through I3–I8.
