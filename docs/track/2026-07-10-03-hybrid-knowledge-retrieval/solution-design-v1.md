---
status: in_progress
scope_type: parent
created: 2026-07-10
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/requirements-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/research/current-state-v1.md
---

# Solution Design v1: Hybrid Knowledge Retrieval

## Design principles

1. Primary graph storage remains the source of truth; indexes are versioned, rebuildable projections.
2. Correct lifecycle behavior precedes ranking or ANN optimization.
3. Every backend has an independent oracle and generation boundary.
4. Embedding network calls happen before/after WAM execution, never inside its instruction loop.
5. Index predicates remain bounded call-time external relations and compose with the existing WAM unifier, proof, cut, and query planning.
6. Node and edge properties have equivalent mutation/index lifecycle semantics while relation identity remains immutable.
7. The current M2 Pro and at most 100K chunks define the delivery scale; no speculative distributed architecture.

## Feasibility

| Area | Assessment | Notes |
|---|---|---|
| property mutation/lifecycle | feasible | existing canonical record plans and raw storage batches provide foundations |
| profile/generation/outbox | moderate | requires coordinated version contracts, but no distributed transaction |
| real BM25 | feasible/moderate | compare embedded mature backend with custom RocksDB postings before selection |
| vector ANN | moderate | embedded crate allowed; canonical vectors make ANN disposable/rebuildable |
| model registry/worker | feasible | current providers/queue exist but require versioned batch job semantics |
| event-time index | feasible | ordered timestamp keys and range seeks |
| valid-time overlap | moderate | dual endpoint indexes/intersection and skew benchmarks required |
| bounded provider cursors | moderate/high | external choice-point state must preserve paging/backtracking/cut |
| hybrid fusion/filtering | moderate | RRF is simple; top-k after Prolog filters needs progressive over-fetch |
| 100K benchmark suite | feasible | staged datasets and deterministic sampling control local cost |

## 1. Canonical entity mutation layer

### Entity identity and property patch

Add shared contracts:

```text
EntityRef = Node(NodeId) | Edge(EdgeId)
PropertyPatch {
  set: Map<Key, Value>,
  remove: Set<Key>,
}
EntityMutation {
  sequence,
  entity,
  entity_version,
  kind: Upsert | Delete,
  changed_fields,
  occurred_at,
}
```

`PropertyPatch` is atomic. Set wins only if a key is not also in `remove`; conflicting input is rejected. Missing removals are successful no-ops. `Value::Null` remains a value.

Expose explicit Rust/JSON and Prolog operations:

```prolog
set_node_property(Node, Key, Value).
remove_node_property(Node, Key).
set_edge_property(EdgeId, Key, Value).
remove_edge_property(EdgeId, Key).
edge_id(Source, Type, Target, EdgeId).
```

`assertz/retract(property/3)` follows the approved first-version generic entity contract: resolve an existing entity ID and reject ambiguity. Edge source/type/target/ID are immutable. Edge updates use entity version/tombstone metadata keyed by entity; the mutation sequence is the indexing source version. Storage record layouts may change directly before first release. Relation identity changes are delete/create.

### Durable mutation outbox

Every primary node/edge write allocates a monotonic storage mutation sequence and atomically writes:

```text
primary records/indexes
entity version/tombstone
mutation outbox record
next-sequence metadata
```

This belongs in `zlf-storage`, below API and Prolog writers, so no write path can bypass it. The outbox is index-agnostic and does not depend on `zlf-index`. At the current single-process RocksDB writer boundary, sequence allocation is serialized and included in the same `WriteBatch`.

Workers tail events in sequence. Each active index target records a contiguous scanned watermark, including irrelevant/skipped events. An older event checks the current entity/tombstone version and cannot resurrect superseded content. Outbox retention advances only after all active target watermarks pass a checkpoint.

Bulk load publishes a bounded rebuild-required event/generation marker rather than millions of per-record jobs; index generation build scans the loaded canonical storage snapshot.

## 2. Index profiles and generations

### Profile artifact

```text
IndexProfileArtifact {
  name,
  version,
  source_hash,
  matcher: NodeLabels | EdgeTypes,
  fields: Map<Field, FieldIndexOptions>,
  created_at,
}
```

Field options contain BM25 analyzer/weight, vector model/chunking, or temporal role. Profiles are immutable and content-addressed. Activation history has an effective mutation sequence, allowing workers to resolve which profiles applied to each event.

Prolog:

```prolog
:- index_profile(Name, Version, Config).
:- activate_index_profile(Name, Version).
```

Equivalent JSON/Rust APIs lower into the same artifact validator/store. These are facade/storage configuration directives, not ordinary assertable facts or core WAM builtins.

### Generation lifecycle

```text
Draft -> Building -> Validating -> Active -> Retired | Failed
```

A generation namespace includes profile/version, backend schema, analyzer/model identity, source snapshot sequence, and checksum/count metadata. Build writes a fresh namespace. Validation checks counts, metadata, and backend-specific probes. Activation changes one storage metadata pointer atomically. Failure leaves the prior active generation readable.

Initial retention recommendation: active + previous successful generation per target, failed build metadata for diagnostics, and bounded dead letters. Exact counts/age become configuration and are verified against local disk budget.

## 3. Indexed document and chunking

```text
IndexDocumentId {
  entity: EntityRef,
  field,
  chunk_id,
}
IndexDocument {
  id,
  source_version,
  content_fingerprint,
  text_or_temporal_value,
  source_range,
  chunk_ordinal,
  chunk_profile,
}
```

The extractor applies active profiles to the canonical current Node/Edge. It accepts explicit chunks or runs a versioned deterministic chunker:

- `whole_field`;
- paragraph/heading-aware;
- fixed tokenizer window with overlap.

A per-entity/profile manifest records currently published document IDs. Reconciliation compares desired versus prior manifests, upserts changed documents, and deletes removed chunks. This makes edits, profile changes, and entity deletion exact and idempotent.

## 4. BM25 design

### Common backend contract

```text
Bm25Backend {
  build_generation(documents, config),
  reconcile(changes),
  search(query, top_k, filters) -> ranked hits,
  explain(document, query),
  validate(),
}
```

Use Tantivy as the initial mainstream embedded backend with a versioned Jieba-compatible analyzer adapter. Keep it behind the common contract and retain independent score tests. A custom RocksDB postings layout is deferred until stable functionality shows a concrete need.

Required scoring contract uses versioned field-aware BM25 with configurable `k1`/`b`, field weight, bounded top-k heap, and stable document-ID tie-break. Prototype token-count indexes are discarded and rebuilt.

## 5. Embedding model and vector design

### Model registry

```text
EmbeddingModelProfile {
  id,
  version,
  provider,
  model_id,
  model_revision,
  dimension,
  metric,
  normalize,
  max_input,
  query_template,
  document_template,
  batch_limit,
  capabilities,
}
```

`bge_m3_dense_v1` maps to Ollama `bge-m3:latest`, 1024 dimensions, and is the initial baseline. Model profiles separate query/document transforms. Dense is the first delivery; sparse and multi-vector flags remain unsupported until a child benchmark approves them.

Evolve the provider boundary conceptually to:

```text
embed_query(profile, text)
embed_documents(profile, batch)
```

Persistent jobs carry source version/fingerprint/profile, claim lease, attempts, next retry, and error class. Batch-capable providers process batches. Permanent failures move to dead letter; stale source versions are acknowledged without writing vectors.

### Vector storage and ANN

Canonical vectors remain in a versioned RocksDB store keyed by document ID + model profile + generation. Writes validate dimension, finite values, model, metric, and normalization. Exact search is always available for small data, diagnostics, and ANN Recall@k.

ANN is optional for the first functional path because exact retrieval remains canonical. Use `hnsw_rs` as the initial embedded derivative if persistence/reopen integrates cleanly. If ANN integration or delete behavior is complex, defer it and ship exact retrieval. Corrupt/incompatible snapshots rebuild from canonical vectors or fall back to exact.

## 6. Temporal design

### Records and encoding

```text
TemporalRecord = Event { at } | Validity { from, to? }
```

UTC instants encode signed microseconds with sign-bit normalization and big-endian bytes so lexicographic order matches time. Date-only event queries map to UTC day half-open ranges.

Generation key families:

```text
event/by_time/<timestamp>/<document-id>
event/by_entity/<entity>/<field>/<record-id>
valid/by_start/<from>/<record-id>
valid/by_end/<to-or-infinity>/<record-id>
valid/by_entity/<entity>/<field>/<record-id>
```

`temporal_on/2` and `temporal_between/3` query events. `valid_at/2` intersects `start <= t` with `end > t`. `valid_overlaps/3` implements `record.start < query.end && record.end > query.start`. The planner chooses/intersects endpoint candidate sets and reports scanned candidates. Skewed and long-open interval benchmarks determine whether coarse interval buckets are needed; do not add them speculatively.

## 7. Retrieval contracts and WAM composition

### Structured facade

```text
RetrievalRequest {
  query_text,
  modes: Lexical | Vector | Hybrid,
  profiles,
  top_k,
  candidate_k,
  threshold,
  temporal_filter,
  graph_filter_goal?,
  minimum_watermarks?,
  explain,
}
RetrievalHit {
  document_id,
  entity,
  field,
  chunk,
  score,
  rank,
  lexical?,
  vector?,
  generation/watermark,
  source_range,
  explanation?,
}
```

The async facade performs query embedding and creates a request-scoped retrieval context before starting WAM. WAM/provider code only reads prepared vectors/handles. Existing synchronous source-node `vector_similar/3` remains available.

### Prolog relations

Existing predicates remain. Add one option-bearing relation rather than many combinatorial arities:

```prolog
retrieve(Query, Options, Entity, Hit).
```

`Hit` is a typed object containing stable document identity, field/chunk, ranks/scores, generation, and optional explanation. Query planning rewrites literal text semantic/hybrid retrieval into an internal prepared query handle before WAM execution. A synchronous API rejects uncached text embedding with a typed error rather than making HTTP calls from WAM.

### Fusion and filtering

Reciprocal-rank fusion is the first hybrid baseline:

```text
RRF(d) = sum_retrievers 1 / (k + rank_r(d))
```

Default `k` is versioned/configurable; raw BM25/cosine scores remain available but are not added directly. Stable document ID breaks ties.

Graph/ACL filters support two plans:

- filter-first: enumerate/bind allowed entities, then score bound documents where selective;
- retrieval-first: page/over-fetch candidates, evaluate the WAM filter goal for each entity, continue until `top_k` accepted hits or candidate exhaustion.

The plan reports candidate count, rejected count, exhaustion, and whether exact top-k was guaranteed. This avoids applying top-k before ACL filtering and silently returning too few hits.

### Bounded provider answers

Keep `facts_for_goal` and current WAM choice points for the first functional path, but enforce explicit backend candidate/page/answer limits before materialization. Report budget exhaustion. Defer a WAM-owned cursor API until stable functionality and measurements show it is necessary.

Proof leaves include index kind, profile/generation, document ID, rank/score, and content fingerprint without copying source text. Tabled rules that consume indexes record index target generation/watermark dependencies; worker publication invalidates affected tables. Unsupported index/table combinations fail explicitly rather than caching indefinitely stale results.

## 8. Observability and consistency API

Expose:

```text
IndexStatus(target) -> generation, source snapshot, watermark, counts, state
IndexJobMetrics -> pending/claimed/retry/dead/stale/lag
IndexQueryMetrics -> latency, candidates, pages, cache/ANN stats
wait_for_indexes(targets, minimum_sequence, timeout)
```

Write responses include source sequence and pending targets. Query responses include used watermarks. A wait timeout reports primary commit plus pending indexes and never pretends the write failed atomically.

## 9. Benchmark architecture

### Workloads

- deterministic EnterpriseKB generator with documents, entities, graph, revisions, events, validity, ACL-style rules, and oracle;
- batch 1: EnterpriseKB + SciFact;
- batch 2: FiQA + license-approved MIRACL zh/en subset;
- batch 3: one HotpotQA/KILT subset + one approved agent-memory dataset.

Every run caps indexed chunks at 100K and records deterministic sampling. Dataset adapters emit canonical nodes/edges/properties plus explicit chunks or profile inputs; they never own private index keys.

### Runner and reports

A runner separates:

```text
source conversion
primary load
profile generation build
embedding generation
BM25/vector/temporal construction
cold/warm/fresh-process queries
mixed mutation/catch-up
quality evaluation
```

Independent Python/reference oracles calculate BM25 fixtures, exact cosine, temporal filtering, graph bindings, relevance metrics, and stale-result counts. Reports include commit/dirty state, machine, dataset/license/checksum, profile/model/chunk config, seed, timings, p50/p95/p99, QPS, RSS, disk, counts, watermarks, failure/retry metrics, and quality. Generated corpora/indexes stay ignored; compact reports are curated under `research/`.

Initial baselines set numeric regression thresholds. No performance claim is accepted without quality/correctness at the same configuration.

## Module ownership

| Crate | Responsibility |
|---|---|
| `zlf-core` | shared entity/property value contracts where storage-neutral |
| `zlf-storage` | atomic node/edge mutations, versions/tombstones, mutation sequence/outbox |
| `zlf-index` | profile/model/chunk/index contracts and BM25/vector/temporal backends |
| `zlf-embed` | async provider/model-profile execution and provider errors |
| `zlf-prolog` | WAM property mutation entry points, index provider paging, proof/table dependency integration |
| `zlf-query` | profile facade/store adapter, workers/coordinator, async retrieval preparation/fusion/filtering |
| `zlf-cli` | profile/index/status/wait/rebuild/retrieval commands and benchmark binary wiring |
| `scripts/` | dataset download/convert/oracles/stress orchestration |

## Rejected or deferred designs

- **Synchronous all-index writes:** rejected because remote embedding would control primary write availability.
- **All textual properties indexed by default:** rejected for relevance, privacy, and lifecycle control; opt-in auto profile remains.
- **One vector per node:** rejected; field/chunk/model identity is required.
- **ANN as source of truth:** rejected; canonical exact vectors are required for validation/rebuild.
- **Mixed event/validity predicate:** rejected as ambiguous.
- **Raw score addition:** rejected; use rank fusion baseline.
- **Remote embedding in WAM:** rejected due blocking/nondeterministic execution.
- **Full security system:** deferred; first benchmark uses explicit graph/rule ACL-style filters.
- **1M/distributed/GPU scope:** deferred by approved 100K local limit.

## Rollback strategy

Every new index uses schema/generation namespaces. Prototype indexes are disposable and need not remain readable. Profile activation and backend selection are metadata pointer changes. ANN can be disabled in favor of exact search. Failed workers preserve primary data and outbox records. Before first release, property/storage record layouts may change directly. Generation rollback applies only to generations created under the new implementation.
