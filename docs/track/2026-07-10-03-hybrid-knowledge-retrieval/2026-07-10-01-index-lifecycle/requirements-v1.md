---
status: done
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-10
version: 1
---

# Stage 01 Requirements: Index Identity and Lifecycle

## Goal

Create one durable, observable mutation and generation contract shared by BM25, vector, and temporal indexes before replacing their internal algorithms.

## Requirements

- Define stable `IndexEntityRef::{Node, Edge}`, `IndexDocumentId(entity, field, chunk)`, and source version/content fingerprint.
- Accept explicit adapter chunks and raw field text with a versioned `ChunkingProfile`; provide deterministic whole-field, paragraph/heading-aware, and fixed-token-window baseline chunkers while allowing rich-format adapters to own semantic splitting.
- Define immutable versioned `IndexProfile` artifacts matching node labels or edge types, with explicit per-field BM25/vector/temporal options; support Prolog directive and equivalent JSON/Rust entry points through one store/lowering path.
- Define versioned analyzer, embedding model/dimension, temporal schema, and physical key metadata.
- Add explicit node/edge set/remove/atomic-property-patch operations. Generic property dynamic writes resolve existing entities and reject ambiguous IDs; edge source/type/target/ID remain immutable, and edge identity lookup is exposed.
- Emit idempotent index upsert/delete jobs from every supported storage mutation path, including node/edge property patches, Prolog writes/retracts, and bulk/import workflows.
- Prevent an old/retried job from overwriting a newer source version.
- Support pending/claimed/completed/retryable-failed/dead-letter or equivalent durable states, bounded retries, recovery after process failure, and batch processing.
- Publish per-index generations atomically and expose a consistency watermark.
- Support online/offline rebuild into a new generation, validation, resume, and final activation.
- Use durable eventual consistency by default. Expose per-index generation/watermark and allow callers to wait for selected indexes to reach a minimum source version with a timeout; timeout preserves the committed primary mutation and reports pending indexes.
- Provide index inventory, lag, failure, generation, and rebuild metrics.

## Acceptance

- Insert, node/edge property update, property removal, node deletion/cascade, edge deletion, and replay produce exactly the expected live index documents.
- A stale job cannot resurrect old content after a newer update/delete.
- Killing a worker between write and acknowledgement is idempotently recoverable.
- Rebuild failure leaves the prior generation readable; completion activates only a validated generation.
- Reopen preserves jobs, generations, and watermark.
- All write paths have integration tests or explicitly documented unsupported outcomes.

## Non-goals

- Selecting BM25/ANN algorithms.
- Rich-format parsing beyond the confirmed baseline chunkers.
- Cross-RocksDB atomic transactions; use durable versioned jobs/generations instead.

## Design-time decision

Finalize bounded retention for retired generations and dead-letter jobs without weakening rebuild rollback or operability.
