---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/requirements-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/solution-design-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-01-index-lifecycle/requirements-v1.md
---

# Stage 01 Solution Design v1: Index Identity and Lifecycle

## Scope and feasibility

This stage delivers parent P0–P2 foundations only. It does not replace BM25, vector, or temporal algorithms and does not add provider cursors or hybrid ranking.

| Area | Assessment | Reason |
|---|---|---|
| shared serializable contracts | feasible | first-version modules and schemas |
| atomic node/edge mutation | moderate | existing writes must be consolidated into one RocksDB batch |
| monotonic outbox | moderate | RocksDB is single-process-exclusive; an in-process write mutex can serialize sequence allocation |
| schema initialization | feasible | no pre-release database migration or old-format open path is required |
| profiles/chunks/generations | moderate/high | new stores and coordinator state machines, but all are rebuildable projections |
| Prolog/JSON mutation APIs | moderate | current lowering is centralized but generic property resolution is node-only |

Requirements and acceptance are sufficient. No requirement must return to discovery.

## Design principles

1. Graph records are canonical; all search state is a disposable, generation-scoped projection.
2. One canonical storage mutation batch changes primary records, graph indexes, entity state, sequence metadata, and outbox together.
3. `Edge`, `Node`, and index serialized layouts may change directly to the clean first-version schema.
4. Idempotent patch no-ops emit no event; every committed create/update/delete operation emits exactly one ordered event for each affected entity.
5. Storage owns mutation truth but remains independent of `zlf-index`.
6. Profiles and model artifacts are immutable; activation and generation pointers are the mutable state.
7. Crash recovery is proven with deterministic fake targets before production index backends consume the lifecycle.

## 1. Contract ownership

### `zlf-core`

Add storage-neutral contracts:

```text
EntityRef = Node(String) | Edge(String)
PropertyPatch { set: BTreeMap<String, Value>, remove: BTreeSet<String> }
```

`PropertyPatch::validate` rejects a key present in both sets. Empty patches are valid no-ops. `Value::Null` is stored as a value and never means removal.

### `zlf-storage`

Add versioned, serde-compatible mutation contracts:

```text
MutationSequence = u64
EntityState { entity, source_version: MutationSequence, deleted }
MutationKind = Upsert { changed_fields } | Delete | RebuildRequired { bulk_id } | ConfigurationChanged { namespace, artifact_ref }
MutationEvent { schema_version, sequence, entity?, source_version, kind, occurred_at }
MutationReceipt { sequence?, entity_versions }
```

A source version is the sequence of that entity's latest effective event. It gives node and edge records identical stale-write semantics and supersedes reliance on `Node.current_version` for indexing. Existing node version history may be simplified directly if the first-version storage design no longer needs it.

### `zlf-index`

Add the contracts consumed by later backends:

```text
IndexDocumentId { entity, field, chunk_id }
IndexDocument { id, source_version, fingerprint, source_range, ordinal, chunk_profile, payload }
IndexProfileArtifact / FieldIndexOptions
EmbeddingModelProfile
GenerationId / GenerationMetadata / IndexStatus
RetrievalRequest / RetrievalHit / IndexMetricsSnapshot
```

Enums are tagged and all persisted artifacts carry `schema_version`. IDs use canonical length-prefixed binary encoding internally; display/JSON forms remain typed objects. No key relies on colon-delimited user IDs.

## 2. Canonical storage mutation

Introduce an internal `MutationPlan` containing checked preconditions, RocksDB puts/deletes, and zero or more entity events. All public graph mutations compile a plan while holding `Storage`'s write mutex, then commit one `WriteBatch`:

- create/update/delete node;
- add/remove labels;
- create/update/delete edge;
- node/edge property patch;
- cascade deletion;
- Prolog fact writes/retracts through those methods.

The write mutex covers read-modify-write, sequence range allocation, and batch commit. RocksDB already prevents multiple processes from opening the same database; therefore this provides monotonic allocation for the supported process model without a distributed transaction service.

Keys are versioned binary namespaces under reserved metadata prefixes:

```text
meta/index-lifecycle/schema
meta/index-lifecycle/next-sequence
entity-state/<kind>/<length-prefixed-id>
outbox/<big-endian-sequence>
bulk-session/<id>
```

For a cascade, one batch allocates one sequence per deleted edge followed by one for the node, writes all tombstones/events, and removes all canonical records and graph indexes atomically. Consumers only depend on sequence order, not wall-clock order.

No-op label additions, missing property removals, and patches that produce no property change return a receipt without a sequence. The existing full `update_node` API remains a committed replacement operation and preserves its version/event behavior even when the supplied map is equal. Deletes retain an `EntityState` tombstone so replayed old events cannot resurrect the entity.

### Property and edge identity APIs

Storage exposes:

```text
patch_node_properties(id, patch)
patch_edge_properties(id, patch)
resolve_entity(id) -> missing | node | edge | ambiguous
get_edge_ids(source, type, target) -> ordered IDs
```

Set/remove convenience methods lower to patches. Generic `property/3` resolves an existing node or edge and errors on ambiguity or absence; it no longer creates a node implicitly. Explicit node/edge predicates remain unambiguous. Relation identity is immutable, so changing source/type/target is delete plus create.

`edge_id/4` is a read relation over the adjacency index and may return multiple stable IDs if API-created parallel edges share a triple. Results are ordered by edge ID.

## 3. Schema initialization and bulk behavior

A newly created database initializes lifecycle schema metadata and sequence zero. There is no old database bootstrap, migration marker, or legacy serialized-record fixture before first release. `open_existing` supports only databases created by the new schema and fails clearly on unknown schema versions.

### Bulk load

Raw key APIs remain for internal metadata/rule/table storage but are not graph mutation APIs. Bulk graph loading uses a durable bulk session:

```text
Started -> Writing(checkpoint) -> Finalizing -> Complete
```

Record batches update the session checkpoint. Finalization atomically publishes one bounded `RebuildRequired` event and marks the session complete. Reopen reports unfinished sessions and requires resume or abort; it never silently claims indexes are current. Ordinary JSON imports continue through canonical per-entity mutations unless explicitly switched to bulk mode.

## 4. Outbox consumption and stale suppression

`zlf-storage` provides ordered outbox reads and entity-state checks only. It has no dependency on index backends. In `zlf-query`, each configured target has durable state:

```text
TargetState { scanned_watermark, published_watermark, generation, status }
Job { target, event_sequence, state, lease_until, attempts, next_attempt, error_class }
```

Workers scan every event in order. Irrelevant events advance `scanned_watermark`. Before publishing a document, a worker compares event source version with current `EntityState`; stale events are acknowledged without writing. Claim expiration makes process death replay-safe. Target reconciliation and watermark advancement are committed only after the idempotent fake target operation succeeds.

Retry defaults are versioned configuration: 8 attempts, exponential backoff with bounded deterministic jitter, then dead letter. Permanent validation errors go directly to dead letter. Error payloads are size-limited and redact source content and credentials.

Outbox records are retained until every active target's contiguous scanned watermark passes them. With no active targets, they are retained. Checkpoint compaction keeps the latest retained floor and aggregate counters.

## 5. Profiles, chunks, and generations

### Artifact and activation store

One validator/store path accepts Rust values. JSON requests and Prolog directives only parse/lower into that path. Artifact identity is `(name, version, source_hash)`; recreating the same name/version with different content is rejected. Validated artifact envelopes and activation records live in the primary storage DB, not a backend DB. A `zlf-query` store adapter serializes `zlf-index` contracts and calls a storage-neutral `commit_projection_config(namespace, artifact_records, activation_ref)` operation. That operation allocates an effective mutation sequence and atomically stores the opaque records plus a target-agnostic `ConfigurationChanged` outbox event. Thus profile activation cannot race graph mutation ordering, while `zlf-storage` does not depend on `zlf-index` types.

Profiles explicitly match node labels or edge types and fields. `auto_text_all_v1` is available only by explicit activation. Model/analyzer/temporal/key schema identities are required whenever the corresponding field option is enabled.

### Chunking

Versioned deterministic chunkers operate on normalized UTF-8:

- whole field;
- paragraph/heading-aware split with source byte ranges;
- fixed tokenizer-window with configured size and overlap.

Explicit adapter chunks pass the same validator. Chunk IDs derive from profile/version, field, ordinal, source range, and content fingerprint. Extraction sorts fields and chunks deterministically. A manifest per entity/profile enables exact upsert/delete reconciliation.

### Generation state machine

```text
Draft -> Building -> Validating -> Active -> Retired
                         \-> Failed
```

Build captures a storage sequence and persists checkpoints. Validation records counts, checksums, and fake-target probes. One metadata batch activates only a validated generation and retires the prior active generation. Failed builds leave the active pointer untouched. Reopen resumes `Building` generations or reports `Validating` generations for deterministic revalidation.

Default retention is:

- active plus the previous successful generation per target;
- failed generation metadata for 30 days, capped at 100 entries per target;
- dead letters for 30 days, capped at 10,000 per target;
- aggregate failure counters retained after payload pruning.

Limits are configurable. Pruning never removes the active/previous generation or an outbox event still required by an active target watermark.

## 6. Status, waiting, and observability

Expose Rust first, then JSON/CLI wrappers:

```text
index_status(targets)
wait_for_indexes(targets, minimum_sequence, timeout)
start/resume/validate/activate/rebuild generation
index_inventory / job_metrics / generation_metrics
```

A write receipt reports committed sequence and pending active targets. Waiting observes target published watermarks with a deadline; timeout returns a typed result containing committed sequence and pending targets, never rolls back or reports the primary mutation as failed.

Metrics include document/chunk counts supplied by targets, pending/claimed/retry/dead/stale counts, lag, generation state, build checkpoint, watermark, and last successful rebuild. Stage 01 tests use deterministic counters rather than latency claims.

## 7. API and Prolog surface

Rust storage and `ZlfDatabase` expose explicit patch methods. JSON-over-STDIO adds node/edge set/remove/patch requests, edge-ID lookup, profile activation, status, wait, and rebuild requests through focused handler modules.

Prolog adds write builtins:

```prolog
set_node_property/3, remove_node_property/2
set_edge_property/3, remove_edge_property/2
edge_id/4
```

`assertz(property/3)` and `retract(property/3)` use generic entity resolution. Mutation completion invokes existing selective table invalidation using the changed entity/predicate keys; the storage outbox remains independent of table storage.

## Alternatives

### Facade-owned outbox — rejected

It misses direct WAM/storage writes and cannot atomically commit with primary graph records.

### One outbox job per physical index — rejected

It couples `zlf-storage` to backend configuration and makes profile activation history difficult to replay. One canonical event stream plus per-target state is simpler and rebuildable.

### Duplicate entity-local and outbox source versions — rejected

The mutation sequence is the indexing source version for both nodes and edges. A second edge-only indexing version would add ambiguity; storage record layouts remain free to evolve before first release.

### Keep independent DB puts and repair on startup — rejected

Repair cannot distinguish committed intent from partial mutation and does not satisfy exact event acceptance.

### Unbounded retention — rejected

It violates local operability. Watermark-gated outbox compaction and bounded diagnostic retention preserve recovery while bounding nonessential payloads.

## Verification strategy

1. Serialization snapshots and old-database open fixture.
2. Contract serde/key round trips, invalid profile/model/patch tests, deterministic chunk snapshots.
3. Storage integration tests for every mutation, exact batches as observed through canonical state plus outbox, edge ambiguity, and cascade ordering.
4. Threaded sequence-allocation test and process reopen test.
5. Crash-point tests before target write, after target write, and before acknowledgement using the fake target.
6. Stale update/delete replay and watermark-contiguity property tests.
7. Generation build failure, resume, validation, activation, rollback, and retention tests.
8. WAM/JSON integration tests for explicit and generic property behavior plus selective table invalidation.
9. Focused crate tests followed by workspace quality gates.

## Risks and rollback

- **Mutation refactor regression (high):** retain characterization tests; land node/edge patching before enabling consumers. Roll back by disabling lifecycle consumers, not by reverting committed graph records.
- **Sequence/schema mismatch (high):** schema-version lifecycle values created by the new implementation and fail closed on unknown schemas.
- **Bulk interruption (medium):** durable sessions expose incomplete state and force resume/abort.
- **Generation disk growth (medium):** enforce retention only after successful activation; never delete current rollback generation early.
- **Profile scope creep (medium):** Stage 01 fake target validates extraction/lifecycle only; backend-specific scoring and ANN fields remain opaque validated options.
