---
status: in_progress
owner: kimmy
updated: 2026-07-08
source_requirements: docs/track/zlf/requirements-v1.md
---

# Change Note 006: Deep Prolog/DB Integration, Persistent Rules, SSE, Markdown Import

## Trigger

User clarified follow-up scope:

1. External knowledge base source is a Markdown folder: `~/workspace/docs/wiki/content/`.
2. HTTP JSON Streaming should use SSE.
3. Rules must persist and should be indexed/optimized.
4. Prolog engine should treat database contents as the fact/query space at the lower engine layer, not as a separate pre-query followed by reasoning.

## Decision

Proceed with an incremental implementation:

- Persist rules under indexed RocksDB keys by predicate, and load them when a `QueryPlanner` opens.
- Preserve multiple clauses per predicate.
- Keep database-backed facts resolved inside the Prolog engine as predicate/fact resolution, using storage/indexes lazily per predicate.
- Add SSE endpoint for command responses as the HTTP streaming foundation.
- Add Markdown folder import that creates deterministic document nodes from `.md`/`.markdown` files.
- Keep `FactProvider` read-side only. Put write-time BM25/embedding/temporal updates behind writer hooks or an async ingest pipeline; synchronous BM25 indexing is acceptable for string properties, while embedding generation should remain optional/asynchronous behind an embedder hook.

## Deferred

- Rich Markdown chunking/entity extraction/LLM relation extraction.
- Full SSE streaming of large query result chunks.
- Advanced rule indexing beyond predicate-key indexing.
- Cost-based Prolog predicate ordering.
- Synchronous embedding generation during Prolog fact writes; use write-side hooks/async pipeline instead.
