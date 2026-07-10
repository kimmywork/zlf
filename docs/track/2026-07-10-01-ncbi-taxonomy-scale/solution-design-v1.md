---
status: done
scope_type: parent
created: 2026-07-10
version: 1
---

# NCBI Taxonomy Scale Solution Design

## Goal and source

Implements `requirements-v1.md` and the four stages in `scope-map.md`, while extending—not replacing—the positive tabling design in `docs/track/zlf-kernel-enhancements/research/tabling-and-incremental-tabling.md`.

## Design principles

1. Canonical ground Prolog facts are the audit/replay boundary.
2. One semantic lowering path serves normal and bulk writes.
3. Internal RocksDB keys remain owned and versioned by `zlf-storage`.
4. Compilation and loading are separate, measurable, resumable phases.
5. Large-graph evaluation must push bindings to indexes and avoid relation materialization.
6. Correctness is established independently of the implementation under test.
7. Generated datasets, packs, databases, and reports are local artifacts unless explicitly curated.

## Feasibility pre-screen

| Area | Rating | Reason |
|---|---|---|
| Streaming DMP conversion | Feasible | Input schemas are fixed; nodes/names are confirmed monotonic by tax ID. |
| Restricted fact parsing | Feasible | Existing typed parser can parse individual facts; a streaming statement reader avoids whole-file memory. |
| WriteBatch packs | Feasible | Storage already owns deterministic primary/index keys; RocksDB supports bounded batches. |
| Semantic parity | Moderate | Existing writer mixes parsing, merge, validation, and execution; lowering must be extracted carefully. |
| Bound-aware providers | Moderate | Storage indexes exist, but current provider materializes broad relations before WAM execution. |
| Bound tabled recursion | Moderate | Current fixed-point MVP terminates but computes too broadly; query-seeded SCC/delta work is required. |
| Full 2.86M scale | Moderate | Local machine has 32 GiB RAM and 179 GiB free; staged runs and disk preflight are required. |

## Considered approaches

### A. NCBI script writes RocksDB keys directly — rejected

Fast to prototype but duplicates key encoding, bypasses storage invariants, and couples the source adapter to private schema.

### B. Parse PL and call `apply_fact` per record — rejected for bulk path

Correct but preserves read-before-write, per-record serialization/index writes, and registry overhead. Retained only as the semantic-parity oracle on small fixtures.

### C. Shared lowerer plus versioned bulk pack — chosen

`DMP -> PL -> restricted parser -> FactMutation -> StorageRecordPlan -> pack -> WriteBatch`. It is reusable, inspectable, testable, and allows a later SST backend without changing source facts.

### D. Direct WAM SLG continuation suspension — deferred

Long-term attractive, but positive bound worklist tabling is lower risk for the taxonomy tree. Revisit only if metrics show the wrapper cannot meet workloads requiring general nested tabled calls.

## Deliverable architecture

```text
scripts/ncbi_taxonomy_to_facts.py
  dmp readers -> deterministic sharded .pl + source manifest

zlf-prolog::bulk
  StatementReader
  RestrictedFactParser
  FactLowerer<Term -> FactMutation>
  FactPackCompiler

zlf-storage::bulk
  StorageRecordPlan
  StorageKeyEncoder (schema versioned, crate-owned)
  FactPackManifest / shard reader-writer
  BulkStorageLoader (WriteBatch)

zlf-index
  prefix-bounded scans
  bounded BM25 batch writer/search

zlf-prolog::wam::providers
  bind-aware goal lookup
  provider metrics

zlf-prolog::wam::tabling
  TableKey / hot TableStore / RocksTableBackend
  SCC selection
  query seed and delta worklist
  resource/metrics contract

scripts/ncbi_taxonomy_oracle.py
scripts/run_ncbi_stress.py
  correctness + tiered metrics/report
```

## Contract-first interfaces

### Fact mutation

```rust
enum FactMutation {
    EnsureNode(Node),
    EnsureEdge(Edge),
    AddLabels { id: String, labels: Vec<String> },
    SetProperty { id: String, key: String, value: Value },
}

trait FactLowerer {
    fn lower(&self, fact: &Term) -> WamResult<FactMutation>;
}
```

The normal writer executes this mutation against current state. Bulk compilation accepts an initial-load subset and converts it to storage records. Unsupported merge-dependent shapes fail explicitly in pack v1 rather than silently changing semantics.

### Storage records

```rust
struct StorageRecord {
    key: Vec<u8>,
    value: Vec<u8>,
}

struct StorageRecordPlan {
    primary: Vec<StorageRecord>,
    indexes: Vec<StorageRecord>,
    versions: Vec<StorageRecord>,
}
```

Key construction and serialization are private to `zlf-storage`; normal writes and bulk plans call the same helpers.

### Pack manifest

```rust
struct FactPackManifest {
    format_version: u32,
    storage_key_version: u32,
    source_checksums: BTreeMap<String, String>,
    shard_checksums: BTreeMap<String, String>,
    fact_counts: BTreeMap<String, u64>,
    record_count: u64,
    complete: bool,
}
```

A pack is loadable only when versions match, checksums pass, and `complete` is true. A target database receives its import completion marker last.

### Provider access

```rust
trait FactProvider {
    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>>;
}
```

Storage implementation dispatches bound modes to exact/outgoing/incoming/prefix indexes. Metrics distinguish indexed rows from full scans. The initial API remains materialized `Vec<Term>`; streaming cursors are deferred until bound workloads demonstrate need.

### Table metrics

```rust
struct TableMetrics {
    cache_hits: u64,
    cache_misses: u64,
    iterations: u64,
    inserted_answers: u64,
    duplicate_answers: u64,
    provider_calls: u64,
    provider_rows: u64,
}
```

## Taxonomy fact model

- Taxon: one `node(tax_ID, [taxon], {...})` with scientific name, rank, division, genetic-code fields, and grouped names.
- Parent: `taxonomy_parent(tax_CHILD, tax_PARENT)`; root self-parent is omitted from recursive traversal and retained as metadata.
- Merge: `merged_into(tax_OLD, tax_NEW)`.
- Deleted: `deleted_taxon(tax_ID)`.
- Name lookup: selected exact multivalue property/name index generated from grouped names; fuzzy lookup uses optional BM25 and is not required for initial graph correctness.

Taxonomy distance is `depth(A)+depth(B)-2*depth(LCA(A,B))` with edge count as the unit.

## Increment dependency graph

```text
I1 key encoding + prefix scans
 ├─> I2 shared fact lowerer and parity tests
 │    └─> I3 pack compiler/loader
 │         └─> I4 DMP fact generator
 └─> I5 bind-aware provider
      └─> I6 bounded SCC/delta tabling
           └─> I7 taxonomy rules/oracle
                └─> I8 stress tiers/report
```

## Implementation increments and verification

| Increment | Risk | Deliverable | Verification |
|---|---|---|---|
| I1 | medium | Shared key helpers, true prefix iteration, WriteBatch primitive | storage unit/integration tests and scan counters |
| I2 | medium | `FactMutation` lowerer used by normal writer | existing ISO/dynamic tests plus normal-vs-plan parity fixture |
| I3 | high | Restricted statement stream, pack manifest/shards, loader | malformed/non-ground/checksum/version/idempotency tests |
| I4 | medium | Streaming NCBI converter and fixture | golden facts, deterministic checksums, bounded-memory behavior |
| I5 | high | Bound-aware storage provider | outgoing/incoming/exact tests proving no broad scan |
| I6 | high | SCC/query-seeded/delta table evaluator and metrics | cycles, left recursion, variants, duplicate paths, limits |
| I7 | medium | Taxonomy rules and Python oracle | fixture and sampled real-data equality |
| I8 | medium | Tier runner/reports | automated 10K/100K, opt-in 1M/full |

## Challenge and simplifications

- A fully general PL compiler is unnecessary and risky; pack v1 is restricted to ground writable facts.
- Raw text KV and public key APIs are rejected; packs are versioned and storage-owned.
- SST ingestion, compression, vector embeddings, and live WAM-frame persistence are deferred. Complete table answers and stale metadata in RocksDB are mandatory before large-scale stress runs.
- Full dataset is not a normal CI requirement. CI uses fixtures/small tiers; large runs are ignored/opt-in.
- The 482万 names remain grouped values initially to avoid doubling graph entities; exact index support is built around the grouped representation.

## Acceptance mapping

- R1/R7 -> I4/I7.
- R2/R3/R4 -> I1-I3.
- R5 -> I1/I5.
- R6 -> I5/I6.
- R8 -> I7/I8.

## Rollback

Each increment lands with focused tests. Bulk loading targets a new database and never mutates a production path without explicit command. Pack/schema mismatch fails before writes. High-risk I3/I5/I6 can be reverted independently while preserving canonical generated facts and baselines.
