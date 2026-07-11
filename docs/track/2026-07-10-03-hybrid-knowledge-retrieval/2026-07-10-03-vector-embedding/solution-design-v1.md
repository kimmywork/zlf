---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-03-vector-embedding/requirements-v1.md
---

# Stage 03 Solution Design v1: Vector and Embedding Retrieval

## Scope and dependency

Stage 03 consumes Stage 01 document/profile/generation/job contracts. Canonical vectors are keyed by `(generation, model profile, IndexDocumentId)`; ANN is a rebuildable derivative. Dense vectors ship first. Sparse and multi-vector capability flags validate as unsupported.

## Design principles

1. Canonical exact vectors are the correctness oracle and ANN rebuild source.
2. Model, revision, dimension, metric, normalization, and transform identity never mix implicitly.
3. Provider/network execution is async and outside the WAM loop.
4. Retrieval quality and embedding throughput are measured separately.

## Model and provider contracts

```text
EmbeddingModelProfile { id, version, provider, model_id, revision, dimension,
  metric, normalize, max_input, query_template, document_template,
  batch_limit, capabilities }
EmbeddingJob { document_id, source_version, fingerprint, model_profile,
  expected_dimension, attempts, lease/retry timestamps }
EmbeddingProvider { embed_query(profile,text), embed_documents(profile,batch) }
```

`bge_m3_dense_v1` maps to Ollama `bge-m3:latest`, 1024 dimensions, cosine, normalized dense output, while remaining an ordinary registry artifact. Logs retain IDs/error classes and sizes, not API keys or source text.

Document text is transformed before provider calls; returned vectors must have exact dimensions, finite components, permitted zero-vector behavior, and configured normalization. A stale source version is acknowledged without vector publication. Fingerprint dedupe reuses only vectors with identical model-transform identity.

## Canonical exact store

Versioned RocksDB keys use canonical binary IDs. Values carry model/profile/schema/fingerprint/source version and vector bytes. Upsert/delete uses Stage 01 manifests and generations. Exact search streams only the selected model/generation, computes f64 accumulation for cosine/dot product, applies threshold/source inclusion, uses a top-k heap, and breaks ties by document ID. Zero vectors are rejected for cosine at ingestion; NaN/Inf always fail.

## ANN choice

Canonical exact search ships first and is sufficient for functional delivery. Use `hnsw_rs` as the initial embedded ANN derivative if its current persistence API integrates cleanly behind `AnnBackend`; otherwise defer ANN without blocking the stage. Updates/deletes may rebuild a generation or use tombstones. ANN corruption/version mismatch falls back to exact.

## Query integration

Vector requests specify query vector or prepared query handle, model profile, generation, top-k, threshold, metadata filters, and source inclusion. Async facade embeds query text first and passes an immutable request-scoped handle to WAM. Existing source-node similarity resolves all matching indexed documents explicitly; it never picks an arbitrary vector.

## Verification

- Independent cosine/dot oracle with ties, dimensions, zero, NaN/Inf, model isolation, update/delete/replay/reopen.
- Deterministic precomputed vectors in CI.
- Embedding worker crash/lease/retry/dead/stale/batch tests via fake provider.
- ANN candidate reports and fresh-process Recall@k.
- Opt-in Ollama `bge-m3:latest` document/query throughput and quality; network tests never replace deterministic gates.

## Risks and rollback

- **ANN dependency instability:** selection is evidence-gated; exact fallback is mandatory.
- **Model alias drift (`latest`):** store reported/model revision metadata and treat changed identity as a new generation.
- **Memory at 100K×1024:** benchmark peak RSS before selection; cap builds to approved local tier.
- **Rollback:** disable ANN, reactivate prior generation, replay durable jobs; never discard primary graph data.
