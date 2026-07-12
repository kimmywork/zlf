# Review Feedback Report

## Metadata

- **Reviewer**: self-performed; no independent reviewer subagent was available
- **Phase reviewed**: Stage 03 vector and embedding implementation V0–V5
- **Artifacts inspected**: requirements/design/plan/change notes, exact/vector contracts, lifecycle target, job store/worker, model registry, WAM/query/CLI paths, tests, benchmark and Ollama evidence
- **Prior phases considered**: Stage 01 lifecycle and Stage 02 BM25 delivery
- **Review date**: 2026-07-11

## Summary

- **Total open issues**: 0
- **Critical**: 0
- **Major**: 0
- **Minor**: 0
- **Verdict**: pass

## Resolved during review

1. The node-only `VectorIndex`, direct CLI writes, WAM writer hooks, and prototype queues bypassed canonical document/model/generation identity. They and their active tests were removed rather than retained as compatibility aliases.
2. The first embedding worker boundary was synchronous. It is now async and directly adapts `zlf_embed::EmbeddingProvider`, keeping remote HTTP outside WAM.
3. Source-node similarity previously chose one node vector and returned the source itself. It now evaluates every source chunk in the active exact generation/model, excludes the source entity, aggregates target scores, and sorts deterministically.
4. Ollama used the legacy single-input endpoint. It now uses OpenAI-compatible `/v1/embeddings`, true batches, response index ordering, cardinality/status validation, deterministic mock protocol coverage, and a successful local 1024-dimensional gate.

## Accuracy and validity

- Exact cosine/dot uses f64 accumulation and matches an independent oracle.
- Vector/profile/model/generation identities are explicit; dimension, finite, zero, metric, normalization, revision and capability mismatches are rejected.
- Backend publication precedes durable completion, so lease-expiry replay is idempotent; changed content removes stale vectors before new jobs are processed.
- Job diagnostics contain bounded error classes, not source text, response bodies, credentials, or API keys.
- ANN was not represented as implemented: `hnsw_rs` constraints and deferral criteria are recorded in change note v4.

## Consistency

- Graph mutations still originate from canonical storage/outbox events.
- WAM remains the only Prolog runtime and `FactProvider` remains read-side only.
- No old vector command, serialized path, compatibility alias, or second indexing queue remains.
- Exact and Ollama benchmark claims are explicitly synthetic/smoke claims, not semantic-quality claims.

## Open questions

None blocking Stage 03. Resolving provider aliases such as Ollama `latest` to a digest/revision remains an operational hardening follow-up; the current profile keeps revision identity explicit and must be versioned when deployments resolve a changed revision.
