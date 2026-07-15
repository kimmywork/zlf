---
status: done
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
version: 1
---

# Optional vector embedding/index strategy requirements

## Elevator pitch

Make vector embedding an explicit opt-in database capability rather than an unconditional cost. When enabled, operators choose exact or HNSW retrieval; HNSW remains an asynchronous immutable derivative of exact vectors and transparently falls back to exact on missing/corrupt ANN state.

## Personas and scenarios

- As a code-knowledge operator, I want embedding disabled so symbol-heavy imports use BM25 and graph relations without remote model, vector storage, or ANN build costs.
- As a semantic-search operator, I want exact vectors enabled for correctness-oriented workloads.
- As a larger semantic-search operator, I want HNSW enabled without blocking reads while replacement generations rebuild.
- As an API user, I want a typed error when a disabled index is requested rather than an empty or silently degraded answer.

## Scope

1. Add persisted/configurable embedding enablement, default `false`.
2. Add vector engine selection: `exact` or `hnsw`; HNSW mode also retains exact as source/oracle/fallback.
3. Integrate durable HNSW search into `ZlfDatabase` as a strategy, not a replacement.
4. Rebuild HNSW asynchronously from a consistent exact-vector snapshot and publish immutable generations atomically.
5. Fall back to exact for missing, incomplete, identity-mismatched, or corrupt HNSW publications.
6. Reject vector profiles, vector queries, vector Prolog predicates, vector/hybrid retrieval modes, embedding jobs, and embedding query generation while embedding is disabled.
7. Document batch-oriented operational guidance: import a knowledge batch, complete embeddings, then request one ANN rebuild/publication.

## Non-goals

- Removing exact RocksDB.
- In-place HNSW update/delete.
- Making HNSW the default.
- Auto-enabling embedding because an Ollama endpoint exists.
- Tree-sitter/code indexing implementation; that work has an independent track.
- HTTP calls from the WAM/provider loop.

## Contracts

```text
embedding.enabled = false | true             # default false
embedding.index_engine = exact | hnsw        # default exact when enabled
```

HNSW configuration uses bounded defaults proven by the frozen benchmark (`M=48`, `ef_construction=400`, `ef_search=2048`) and remains overrideable only through validated finite options.

Disabled operations return a typed `IndexUnavailable`/equivalent error identifying `vector_embedding` and the requested operation. Hybrid requests containing vector retrieval fail; they do not silently become lexical-only.

## Acceptance criteria

1. A default-open database does not initialize/catch up embedding jobs or expose vector predicates as usable indexes.
2. Exact-enabled databases preserve existing vector lifecycle and query behavior.
3. HNSW-enabled databases search a valid ANN publication and retain exact vectors.
4. Queries continue through exact fallback while ANN is absent, rebuilding, or corrupt.
5. HNSW rebuild runs outside the query call path and atomically swaps a complete immutable publication.
6. Updates/deletes become visible through exact immediately after embedding publication and through HNSW only after the replacement publication; no stale ANN generation is presented as current.
7. Explicit tests cover disabled errors, exact mode, HNSW reopen, fallback, asynchronous rebuild, and batch rebuild guidance/API.
8. Strict Clippy, tests, formatting, source-size, report, and diff gates pass.

## Verification

- Unit tests for config parsing/defaults/env overrides and typed disabled errors.
- `zlf-index` HNSW persistence/search/corruption tests.
- `zlf-query` strategy/fallback/rebuild/reopen tests.
- CLI/config integration tests proving default disabled and explicit enablement.
- Existing exact-vector lifecycle tests run with explicit exact strategy.

## Risks and rollback

- HNSW memory/build cost: remain opt-in and batch rebuild only.
- Concurrent rebuild races: serialize/coalesce rebuild requests and publish by generation identity.
- Native dump portability: reject incompatible publication and fall back to exact.
- Rollback is configuration to `exact` or `disabled`; exact data is never discarded.
