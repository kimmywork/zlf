---
status: done
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-03-vector-embedding/requirements-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-03-vector-embedding/solution-design-v1.md
---

# Stage 03 Plan v1: Vector and Embedding Retrieval

## Dependency

```text
Stage 01 lifecycle -> V0 model/vector contracts -> V1 exact store/oracle
V0 -> V2 embedding worker
V1 -> V3 optional hnsw_rs integration -> V4 ANN lifecycle
V1 + V2 + V4 -> V5 quality/scale -> V6 review
```

## V0 — Model-safe contracts (medium)

Freeze model profiles, query/document transforms, vector keys/metadata, validation, request/hit, and deterministic precomputed fixtures. Register `bge_m3_dense_v1` as data, not a physical assumption.

## V1 — Canonical exact vectors (high)

Implement multi-document/model generation storage, strict ingestion, exact cosine/dot top-k, threshold/source filtering, deterministic ties, manifests, deletes, replay, validation, and reopen. Compare with an independent oracle.

**Exit:** exact backend is a trusted oracle and fallback.

## V2 — Durable embedding execution (high)

Refactor provider boundary for query/document batches; add leases, attempts, backoff, dead letter, stale suppression, fingerprint dedupe, redacted diagnostics, and fake-provider crash tests.

**Exit:** failed/network embedding cannot lose primary data or publish stale vectors.

## V3 — Initial ANN integration (medium/high)

Integrate `hnsw_rs` behind `AnnBackend` if persistence/reopen is straightforward. Verify results against exact; defer ANN rather than block the functional exact path if integration becomes complex.

## V4 — Derived ANN generations (high)

Implement selected backend build/reopen/validate, tombstone or rebuild policy, fallback on corruption/incompatibility, and fresh-process recall checks.

## V5 — Quality and provider evidence (medium)

Run deterministic local tiers and opt-in Ollama `bge-m3` throughput/quality separately. Freeze budgets only from accepted same-configuration quality/performance evidence.

## V6 — Review and delivery evidence

Run focused/workspace gates and cumulative implementation/acceptance reviews.

## Acceptance mapping

- identity/model/dimension isolation: V0/V1
- exact oracle and lifecycle: V1
- reliable embedding pipeline: V2
- ANN evidence/fallback: V3/V4
- deterministic CI and Ollama opt-in: V1/V5

## Stop conditions

Fall back to exact if ANN integration or reopen fails. Return to design only if model identity cannot be made stable or canonical vectors cannot rebuild derived indexes.
