# Current-State Investigation v1

## Scope

Trace lexical, vector/embedding, and temporal data from write/generation through persistence, WAM provider delivery, facade/CLI consumption, tests, and benchmarks.

Confidence labels follow the repository investigation convention: **confirmed**, **cross-referenced**, **inferred**, and **open**.

## End-to-end map

```text
Node/fact/API text
  -> ZlfDatabase / IndexedStorageFactWriter
  -> BM25Index directly

CLI index_embedding or embedding worker
  -> zlf-embed provider / Embedder
  -> VectorIndex directly

ZlfDatabase::add_node creation timestamp
  -> TemporalIndex directly

BM25Index / VectorIndex / TemporalIndex
  -> IndexFactProvider::facts_for_goal
  -> CompositeFactProvider
  -> WAM external-answer choice points
  -> Prolog conjunction/rule bindings
```

Primary sources: `crates/zlf-query/src/lib.rs`, `crates/zlf-prolog/src/wam/storage/storage_index_writer.rs`, `crates/zlf-prolog/src/wam/providers/index.rs`, and `crates/zlf-cli/src/embed_commands.rs`.

## Confirmed findings

### F1. Indexes are separate RocksDB databases without a shared generation contract

**Confidence: confirmed.** `ZlfDatabase::open` opens `storage/`, `temporal/`, `bm25/`, and `vector/` independently. Writes are not one cross-database atomic transaction and no common generation/watermark is read during query. Source: `crates/zlf-query/src/lib.rs`.

**Impact:** crashes or partial failures can leave primary storage and indexes at different versions; the query response cannot report freshness.

### F2. Current BM25 is not BM25 scoring

**Confidence: confirmed.** `BM25Index::index_texts_batch` stores per-document token counts as `score`; `search` sums those counts. It stores no document count, document length, average length, document frequency, `k1`, or `b`. Source: `crates/zlf-index/src/bm25.rs`.

**Impact:** lexical ranking tests prove token lookup and Jieba tokenization, not BM25 relevance.

### F3. BM25 update/delete lifecycle is incomplete and can retain stale postings

**Confidence: confirmed.** Re-indexing text writes new token postings but does not remove tokens absent from the new text. `remove_all_for_node` scans the whole database and uses substring matching against keys; it is not called by `ZlfDatabase` mutation/retract paths. Sources: `crates/zlf-index/src/bm25.rs`, `crates/zlf-query/src/lib.rs`, and `crates/zlf-prolog/src/wam/builtins/dynamic.rs`.

### F4. Vector search is an exact full database scan

**Confidence: confirmed.** `VectorIndex::find_similar` iterates from RocksDB start, deserializes every vector, computes cosine, sorts all qualifying rows, then truncates. Source: `crates/zlf-index/src/vector.rs`.

**Impact:** it is useful as a small correctness oracle but not a production ANN implementation.

### F5. One vector key can represent only one vector per node

**Confidence: confirmed.** The physical key is `vector:{node_id}`. Model is stored in the value but not used to partition or validate search. Re-indexing another field/chunk/model overwrites the previous entry. Sources: `crates/zlf-index/src/vector.rs` and `crates/zlf-query/src/lib.rs`.

### F6. Dimension/model errors are silently filtered at query time

**Confidence: confirmed.** `find_similar` skips dimension mismatches. `add_entry` does not validate configured dimension, finite values, normalization, or model compatibility. `vector_similar` searches using the source entry but does not filter candidates by the source model. Source: `crates/zlf-index/src/vector.rs`.

### F7. Embedding generation exists in several disconnected paths

**Confidence: cross-referenced.** The CLI can synchronously generate or accept a vector; `IndexedStorageFactWriter` can synchronously call a blocking embedder; and a persistent queue/worker can process jobs. `ZlfDatabase::apply_fact` enables BM25 but not embedding, while `add_node` indexes BM25 only. Sources: `crates/zlf-cli/src/embed_commands.rs`, `crates/zlf-prolog/src/wam/storage/storage_index_writer.rs`, `persistent_embedding_queue.rs`, and `crates/zlf-query/src/lib.rs`.

**Impact:** there is no single documented write/index consistency contract.

### F8. The persistent embedding queue is durable but operationally minimal

**Confidence: confirmed.** Jobs contain ID, node ID, and text. Processing scans all pending jobs, embeds one-by-one, writes a vector, and acknowledges. There is no content/version fingerprint, model ID in the job, claim/lease, attempts, backoff, dead-letter state, batch-provider call, or stale-job suppression. Source: `crates/zlf-prolog/src/wam/storage/persistent_embedding_queue.rs` and `embedding_worker.rs`.

### F9. Temporal queries currently index creation/start date, not active validity

**Confidence: confirmed.** `add_node` creates a temporal entry from `Node.created_at` with no end. Keys are `temporal:{valid_from date}:{node_id}`. `temporal_on` returns entries whose start date exactly equals the requested date. `temporal_between` filters `valid_from` dates in the inclusive range. `valid_to` is stored but ignored by all query methods. Sources: `crates/zlf-query/src/lib.rs`, `crates/zlf-index/src/temporal.rs`, and `crates/zlf-prolog/src/wam/providers/index.rs`.

### F10. Most temporal operations are full scans

**Confidence: confirmed.** Exact-date, range, before, and after methods use `IteratorMode::Start`; only key string filtering distinguishes dates. Source: `crates/zlf-index/src/temporal.rs`.

### F11. WAM composition works functionally but provider results are eagerly materialized

**Confidence: cross-referenced.** `IndexFactProvider::facts_for_goal` returns vectors of terms. BM25 has no top-k; `vector_similar` hard-codes threshold `0.0` and limit `100`; temporal range has no limit. Existing tests prove these relations can join with graph predicates through the normal WAM call path. Sources: `crates/zlf-prolog/src/wam/providers/index.rs`, `crates/zlf-cli/tests/embedding_query.rs`, and `crates/zlf-prolog/tests/index_wam_provider.rs`.

### F12. Existing benchmark evidence is not sufficient for production claims

**Confidence: confirmed.** `scripts/benchmark.py` starts a fresh CLI process per operation, indexes 100 text records, reports averages, has no relevance judgments, no vector retrieval benchmark, no temporal benchmark, and does not record commit/machine/checksums/percentiles beyond limited medians. The ignored wiki pipeline checks only non-empty BM25/vector behavior and may use synthetic embeddings. Sources: `scripts/benchmark.py` and `crates/zlf-prolog/tests/wiki_full_pipeline.rs`.

## Existing strengths to retain

- **Confirmed:** call-time `IndexFactProvider` composition uses the same WAM external relation path as graph/storage providers.
- **Confirmed:** BM25 token prefix seeks stop when the prefix changes.
- **Confirmed:** embedding providers support Ollama, OpenAI-compatible, and HuggingFace APIs, with batch support in the trait.
- **Confirmed:** a persistent queue and worker foundation already exists.
- **Confirmed:** Chinese tokenization and mixed Chinese/English smoke tests exist.
- **Confirmed:** CLI, facade, direct provider, and one graph/vector join test cover multiple public layers.

## Gaps requiring product decisions

### Temporal meaning

**Status: open.** Existing names can mean either "started on/between" or "valid on/overlap." Enterprise records and agent memories often need both event timestamps and validity intervals. The first-class semantics must be chosen before a new key layout or predicate contract is designed.

### ANN backend policy

**Status: open.** The exact scan is a valuable oracle, but million-scale 1024-dimensional retrieval requires an ANN structure or a much more specialized disk layout. Whether an embedded ANN crate is acceptable affects design and schedule.

### Consistency

**Status: open.** Separate index stores suggest durable eventual consistency, but some users may require read-your-write search. A watermark plus optional synchronous wait is a possible contract, not yet approved.

### Chunk ownership

**Status: open.** Current indexes are node-based. Retrieval quality usually requires document chunks and field-aware indexing. The ingestion/index boundary must say who splits and versions chunks.

## Initial risk ranking

| Risk | Severity | Reason |
|---|---|---|
| stale index records after update/delete | critical | produces factually wrong knowledge responses |
| misleading BM25 name/score | high | quality claims do not match implementation |
| vector full scan/model overwrite | high | blocks scale and multi-model operation |
| undefined temporal semantics | high | key design and query correctness cannot be judged |
| unbounded provider materialization | high | memory/latency risk in Prolog joins |
| benchmark without quality oracle | high | speed can improve while retrieval worsens |
| remote embedding variability | medium | conflates provider latency with index performance |

## Recommended next investigation

1. Confirm temporal semantics and index consistency expectations with the product owner.
2. Freeze indexed-document/chunk/version identity and mutation event contracts.
3. Evaluate custom RocksDB BM25 versus an embedded mature text index against lifecycle and Chinese-analysis requirements.
4. Benchmark exact vector search and at least one embedded ANN candidate with deterministic vectors before backend selection.
5. Verify candidate public dataset licenses, checksums, ground truth, and local resource requirements.
