---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-10
version: 1
---

# Stage 01 Requirements: Index Identity and Lifecycle

## Goal

Create one durable, observable mutation and generation contract shared by BM25, vector, and temporal indexes before replacing their internal algorithms.

## Requirements

- Define stable `IndexDocumentId(node, field, chunk)` and source version/content fingerprint.
- Define versioned analyzer, embedding model/dimension, temporal schema, and physical key metadata.
- Emit idempotent index upsert/delete jobs from every supported storage mutation path, including Prolog writes/retracts and bulk/import workflows.
- Prevent an old/retried job from overwriting a newer source version.
- Support pending/claimed/completed/retryable-failed/dead-letter or equivalent durable states, bounded retries, recovery after process failure, and batch processing.
- Publish per-index generations atomically and expose a consistency watermark.
- Support online/offline rebuild into a new generation, validation, resume, and final activation.
- Define read-your-write behavior: default eventual consistency plus an optional wait-for-watermark is the current recommendation, pending confirmation.
- Provide index inventory, lag, failure, generation, and rebuild metrics.

## Acceptance

- Insert, update, property removal, node deletion/cascade, and replay produce exactly the expected live index documents.
- A stale job cannot resurrect old content after a newer update/delete.
- Killing a worker between write and acknowledgement is idempotently recoverable.
- Rebuild failure leaves the prior generation readable; completion activates only a validated generation.
- Reopen preserves jobs, generations, and watermark.
- All write paths have integration tests or explicitly documented unsupported outcomes.

## Non-goals

- Selecting BM25/ANN algorithms.
- Generating chunks inside the engine before ownership is confirmed.
- Cross-RocksDB atomic transactions; use durable versioned jobs/generations instead.

## Open questions

- Default synchronous versus eventual indexing contract.
- Whether source adapters or zlf own chunking.
- Retention policy for old generations and dead-letter jobs.
