---
status: done
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
version: 1
---

# Optional vector embedding/index strategy design

## Chosen model

Separate capability enablement from physical search strategy:

```rust
pub enum VectorIndexStrategy {
    Disabled,
    Exact,
    Hnsw(HnswOptions),
}

pub struct ZlfDatabaseOptions {
    pub vector_index: VectorIndexStrategy,
}
```

`Default` is `Disabled`. `ZlfDatabase::open/open_existing` use defaults; explicit callers use `open_with_options/open_existing_with_options`. CLI converts `zlf-config` values into query options.

HNSW mode always owns an exact store and may own a loaded ANN snapshot:

```text
canonical storage -> embedding jobs -> exact RocksDB
                                      -> async HNSW rebuild -> atomic ANN publication
query -> valid ANN snapshot; otherwise exact
```

This avoids treating ANN as authoritative and makes fallback local and deterministic.

## Disabled capability boundary

`ZlfDatabase` stores vector runtime as `Option<VectorRuntimeParts>`. Disabled open does not bootstrap vector generation, register vector targets, enqueue embedding jobs, or add vector search to `IndexFactProvider`.

A central `require_vector(operation)` helper returns `ZlfError::IndexUnavailable { index: "vector_embedding", operation }`. All public embedding/vector APIs call it. Retrieval preparation rejects vector and hybrid-vector modes before provider execution. Direct Prolog `vector_similar` fails explicitly rather than returning no rows; this requires registering a disabled provider/error path rather than simply omitting the predicate.

Index profiles declaring vector fields are rejected while disabled. BM25-only and temporal-only profiles remain valid.

## HNSW durable derivative

Move `hnsw_rs` from a benchmark-only dependency into `zlf-index` and add `HnswVectorIndex`:

- immutable build input: exact records for one generation/model/version;
- internal `usize` IDs map to serialized canonical `VectorKey` records;
- identity marker binds schema, generation, model revision, dimension, metric, normalization, HNSW parameters, source watermark/count/checksum;
- graph/data/mapping/marker publish via temporary directory rename;
- open validates identity and mapping before search;
- search converts cosine distance to score, reapplies all source/entity/field/metadata/threshold filters, and uses bounded adaptive overfetch when filters are present;
- unsupported metric/configuration rejects build and falls back to exact at the facade.

The first production strategy defaults to the measured `M=48`, `ef_construction=400`, `ef_search=2048`. All limits are finite.

## Asynchronous rebuild ownership

`ZlfDatabase::request_vector_rebuild()` schedules one background rebuild if HNSW is enabled. A shared state machine is held behind `Arc<Mutex<_>>`:

```text
Idle | Building | Ready(identity) | Failed(redacted class)
```

Multiple requests while `Building` coalesce into one pending rerun. The worker snapshots/export exact records before construction, builds outside query locks, atomically publishes, opens the new index, then swaps `Arc<HnswVectorIndex>` under a short write lock. Queries use the old ready snapshot or exact fallback throughout.

Embedding batch processing marks ANN stale and schedules rebuild only when explicitly requested by batch-oriented APIs; per-document mutations do not synchronously rebuild. A convenience `process_embedding_batch_and_request_rebuild` supports end-of-batch operation. Status exposes readiness/building/fallback without source text.

## Query routing

- `Exact`: exact only.
- `Hnsw`, ready and identity-current: ANN.
- `Hnsw`, absent/building/stale/open failure/search failure: exact fallback.
- `Disabled`: typed error.

ANN hits are canonical `VectorHit`s. If filters or requested top-k cannot be satisfied within bounded overfetch, exact fallback supplies correctness rather than claiming exhaustive ANN filtering.

## Configuration

`zlf-config::EmbeddingConfig` gains:

```json
{
  "embedding": {
    "enabled": false,
    "index_engine": "exact"
  }
}
```

Environment overrides:

```text
ZLF_EMBED_ENABLED=true|false
ZLF_VECTOR_INDEX_ENGINE=exact|hnsw
```

Provider/model settings remain dormant while disabled.

## Implementation slices

1. Contracts/config/default-disabled errors and explicit-open APIs.
2. Optional exact runtime and disabled query/profile/provider behavior.
3. Durable `HnswVectorIndex` with focused persistence/search tests.
4. Async facade rebuild, status, exact fallback, and reopen tests.
5. CLI wiring, operational docs, cumulative verification and acceptance.

## Verification and rollback

Each slice runs focused tests. Final gates run workspace tests/strict Clippy/format/size. Rollback is configuration to exact or disabled. ANN directories can be deleted without losing exact records; failed rebuild never replaces a ready publication.
