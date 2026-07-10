---
status: done
scope_type: parent
created: 2026-07-10
version: 1
---

# NCBI Taxonomy Scale Validation Requirements

## Elevator pitch

Build a reusable ground-Prolog-fact compilation and bulk-loading pipeline, optimize deterministic positive tabling for bound large-graph queries, and validate correctness and performance against the full local NCBI Taxonomy dump.

## Users and scenarios

- A knowledge-base builder converts external structured datasets into auditable canonical Prolog facts and loads millions of facts without per-record storage overhead.
- A Prolog user queries taxonomy names, lineage, common ancestors, descendants, and taxonomy-tree distance over cyclic-safe tabled rules.
- A maintainer receives reproducible correctness and performance evidence at 10K, 100K, 1M, and full-data scales.

## Confirmed decisions

1. Taxonomy distance means parent-tree edge distance through the lowest common ancestor, not sequence-derived genetic distance.
2. Source conversion is `DMP -> canonical ground .pl facts`.
3. Bulk ingestion is `ground .pl -> restricted fact compiler -> versioned bulk pack -> RocksDB loader`.
4. Normal writes and bulk compilation share one fact-lowering semantic path; no second storage mapping is permitted.
5. Bulk packs carry format/key-schema versions, source checksums, counts, and shard checksums.
6. Initial loading uses bounded RocksDB `WriteBatch`; sorted SST ingestion is deferred until measurements justify it.
7. Tabling uses a mandatory two-level store: hot tables in memory and complete cold tables in RocksDB. WAM continuations are never persisted; an interrupted `Evaluating` table restarts as `Stale`.
8. Generated `data/` and prompt history remain outside source control.

## Scope

- Canonical NCBI taxonomy fact generator.
- Restricted streaming parser for ground fact files.
- Shared fact-to-storage mutation plans.
- Batched node, edge, version, adjacency, label, and selected property/name indexes.
- Resumable/versioned bulk packs and loading.
- Prefix-bounded storage/index scans.
- Bound-aware provider access and deterministic positive tabling optimization.
- Taxonomy rules, independent oracle, workload runner, metrics, and scale report.

## Non-goals

- Sequence-derived genetic/evolutionary distance.
- General rules or queries in bulk fact inputs.
- Vector embedding of the complete taxonomy dump.
- Negation, aggregation, WFS, answer subsumption, or concurrent SLG consumers.
- Persisting live WAM frames.
- SST ingestion before a WriteBatch baseline demonstrates need.

## Requirements

### R1 Canonical fact artifact

The converter shall stream `nodes.dmp`, `names.dmp`, `merged.dmp`, and `delnodes.dmp` into deterministic, sharded, ground Prolog facts without memory proportional to total input size.

### R2 Fact compiler contract

The compiler shall accept one ground fact per statement, preserve typed values, reject variables/rules/queries/unsupported facts, and emit a deterministic versioned bulk pack.

### R3 Semantic parity

Given the same supported fact set, normal fact writing and bulk loading shall produce equivalent primary records and query-visible indexes.

### R4 Bulk safety

Loading shall use bounded batches, validate manifest/schema/checksums before writes, be idempotent, expose progress, and leave a completion marker only after all shards succeed.

### R5 Indexed access

Bound node/edge/name/property lookups shall use prefix/exact indexes rather than scanning the full RocksDB keyspace. Unbound full scans shall be explicit and observable.

### R6 Scalable tabling

Explicit positive tabled taxonomy predicates shall terminate on cycles, deduplicate answers, restrict evaluation to the target recursive component and bound call, propagate answer deltas, reuse complete variant tables, and enforce resource limits.

### R7 Taxonomy knowledge base

The delivered facts/rules shall support name resolution, parent/child, lineage, descendants, rank filtering, LCA, merged/deleted IDs, same-rank relatives, genetic-code relation queries, and `taxonomic_distance/3`.

### R8 Stress verification

The runner shall measure conversion, compilation, loading, cold/warm query latency, answer throughput, memory, database size, provider reads, table iterations, answer dedupe, and cache hits at available scale tiers. Results shall be checked against an independent taxonomy-tree oracle.

## Acceptance criteria

1. A small fixture round-trips through DMP conversion, fact compilation, bulk load, and Prolog queries with semantic-parity tests.
2. Corrupt, incompatible, non-ground, or unsupported inputs fail before being marked complete.
3. Bound parent/name/property queries do not execute a whole-database prefix-independent scan.
4. Cyclic and duplicate-path tabled tests terminate with unique correct answers; non-tabled behavior remains unchanged.
5. LCA and taxonomy distance match the independent oracle on fixture and sampled real-data cases.
6. The 10K and 100K tiers run automatically; larger tiers are opt-in and produce machine-readable reports.
7. Full project quality gates pass before delivery.

## Verification

- Unit tests for parsing, lowering, key encoding, manifests, checksums, batching, normalization, and table deltas.
- Storage semantic-parity and prefix-scan integration tests.
- Query integration tests for taxonomy rules and invalidation boundaries.
- Independent Python oracle comparisons.
- Scale runner JSON/Markdown reports with environment and dataset checksums.

## Risks and rollback

- Key-format bugs can corrupt databases: load only into a new/explicit target, version packs, validate fully, and write completion last.
- Full closures can explode: require bound calls, SCC scoping, limits, and answer-volume reporting.
- Generated facts can consume substantial disk: shard and optionally stream/compress; preflight free space.
- Changes remain separable by stage and can roll back to commit `e24840c`.
