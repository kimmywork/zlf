---
status: in_progress
scope_type: parent
created: 2026-07-10
version: 1
source_requirements:
  - user discussion 2026-07-10
  - AGENTS.md
  - docs/track/zlf-kernel-enhancements/delivery-record-v1.md
---

# Hybrid Knowledge Retrieval Requirements

## Elevator pitch

Make zlf a practical general knowledge engine by turning its current BM25, vector, and temporal prototypes into correct, lifecycle-safe, scalable indexes that compose naturally with Prolog rules and graph joins, then validate the combined system on reproducible general-knowledge, enterprise-like, and agent-memory workloads.

## Product roles

1. **Enterprise knowledge base** — ingest documents, entities, relationships, and validity periods; support lexical, semantic, temporal, graph, and policy/rule queries.
2. **External knowledge engine for agents** — retrieve relevant memories/evidence with temporal and relationship constraints and return traceable source records for downstream reasoning.
3. **General Prolog with indexed data types** — expose text, vector, and temporal relations as first-class call-time predicates that can join with ordinary WAM goals without moving core semantics into providers.

## Current pain

The active query path wires all three index families into `IndexFactProvider`, but current tests establish only small functional examples. There is no complete cross-index lifecycle contract, quality oracle, scale evidence, hybrid ranking contract, or public knowledge-base benchmark. Current vector and temporal searches scan their RocksDB databases, current BM25 scoring is token-frequency accumulation rather than corpus-normalized BM25, and Prolog index predicates have fixed implicit limits or no limits.

## Personas and scenarios

### Enterprise knowledge engineer

As a knowledge engineer, I want a document/entity update to refresh all configured indexes consistently, so deleted or superseded content cannot remain searchable.

### Agent platform developer

As an agent developer, I want semantic and lexical candidates filtered/joined by graph and temporal rules, so retrieval returns relevant and currently valid evidence rather than isolated nearest neighbors.

### Prolog application developer

As a Prolog developer, I want index predicates with explicit, typed query contracts and deterministic result semantics, so they can participate in conjunctions, rules, cut, proof, and tabling safely.

### Operator

As an operator, I want index build, queue lag, query latency, recall, storage, and consistency metrics, so I can size and monitor a long-running deployment.

## Parent requirements

### R1. Unified indexed-document identity and profile

Every indexed unit must have stable identity independent of a physical index key:

```text
IndexEntityRef = node(NodeId) | edge(EdgeId)
IndexDocumentId = entity + field + chunk_id
```

A versioned immutable `IndexProfile` explicitly matches node labels or edge types and declares per-field BM25 weight/analyzer, vector model/chunking, and temporal role. Production defaults index only declared fields; an opt-in `auto_text_all_v1` profile supports demos. Profiles can be created through `:- index_profile(Name, Version, Config).` or equivalent JSON/Rust APIs, persist through one artifact/store path, build a new generation, and activate atomically only after validation.

The indexing boundary accepts adapter-supplied explicit chunks and raw field text plus a versioned `ChunkingProfile`. zlf provides deterministic whole-field, paragraph/heading-aware, and fixed-token-window baseline chunkers; rich formats may be split by adapters. Chunks retain source entity/field, ordinal, source range, profile/version, and content fingerprint. Index records must carry source version/content fingerprint, analyzer or embedding model identity, and index schema version. Multiple fields, chunks, languages, embedding models, and historical validity records must not overwrite one another accidentally.

### R2. Lifecycle correctness

Node/fact API writes, Prolog dynamic writes/retracts, imports, bulk loads, updates, and deletes must atomically persist the primary mutation, source version, and durable index jobs. Durable eventual consistency is the default: jobs must be idempotent, retryable, observable, and version-checked so an old job cannot overwrite a newer document version. Callers may explicitly wait for selected indexes to reach a minimum source version with a timeout; timeout does not roll back the committed primary mutation and must report pending indexes.

Each index exposes its generation and consistency watermark, and search responses identify the generation/watermark used. Prolog dynamic mutation does not wait for remote embedding inside the WAM loop. The system must support index rebuild, validation, checkpoint/resume, and generation-based atomic publication. Search must never silently mix incompatible analyzer/model/schema generations.

### R3. Real lexical retrieval

BM25 must use document frequency, corpus/document length statistics, configurable `k1`/`b`, deterministic tie-breaking, field/chunk identity, bounded top-k, and update/delete correctness. Chinese and English analysis behavior must be versioned and tested. Phrase/position search and advanced analyzers are optional unless benchmark evidence requires them.

### R4. Scalable vector retrieval

Vector indexing must validate dimensions and model identity, support text-query and source-node-query workflows, and provide exact search as the correctness oracle and first functional backend. A pluggable embedded ANN derivative may be added after the exact path is stable; `hnsw_rs` is the initial choice if integration is straightforward. It must not require an external vector service, and exact RocksDB search remains the oracle/fallback. Search contracts must include top-k, threshold, model/index generation, deterministic tie-breaking, and whether the source item is included.

Embedding models use a pluggable versioned registry. Each profile covers provider, model ID/revision, dimension, metric, normalization, maximum input, query/document prompt or prefix templates, batch limits, and dense/sparse/multi-vector capabilities. Ollama `bge-m3:latest` 1024-dimensional dense embedding is the default and first benchmark baseline, not a hard-coded storage assumption. Embedding generation throughput, failure/retry behavior, batching, dedupe, stale-job suppression, and provider/model metadata must be measured separately from vector retrieval latency.

### R5. Explicit temporal semantics

The first delivery uses two distinct temporal record kinds:

- event time: a UTC instant at which an event occurred;
- valid time: a half-open UTC interval `[valid_from, valid_to)`, with an optional open end.

Storage versions remain available as internal transaction history, but full bitemporal query algebra is deferred. Event and validity records must not be conflated in storage or query results. Existing `temporal_on/2` and `temporal_between/3` represent event-time date/range queries. New `valid_at/2` and `valid_overlaps/3` predicates represent valid-time containment and overlap. Date-only event queries use UTC day boundaries, and all ranges use half-open `[start, end)` semantics. Point, overlap/range, before, and after queries must use ordered index seeks rather than full scans at target scale. Mutation and deletion must remove superseded temporal records.

### R6. Hybrid retrieval and Prolog composition

The facade must support:

- lexical-only, vector-only, and temporal-only retrieval;
- hybrid lexical/vector fusion with an explicit algorithm such as reciprocal-rank fusion;
- graph, property, label, rule, and temporal filters;
- retrieval-first and bound-entity lookup plans;
- score, rank, source field/chunk, model/analyzer generation, and optional explanation metadata.

Raw BM25 and cosine scores must not be added directly without calibration. First-version predicates and option-bearing APIs require an explicit contract and planner visibility; no backward-compatibility aliases are required.

### R7. Call-time execution and bounded materialization

Index predicates remain external read relations called from the WAM path. They must honor bound arguments where useful, expose pushed constraints in query plans, and avoid unbounded `Vec<Term>` materialization. Core ranking and index logic belongs in `zlf-index`/the query facade, not in a second Prolog evaluator.

### R8. Observability and operability

Expose at least:

- documents/chunks/vectors/temporal records per generation;
- pending/running/retried/failed/stale indexing jobs;
- index build/update/delete throughput;
- query count, candidate count, latency percentiles, cache/ANN metrics;
- model/analyzer/schema identity;
- consistency watermark and last successful rebuild.

Backup/reopen/rebuild and corrupt/incompatible metadata behavior must be tested.

### R9. Correctness and quality evaluation

Use independent oracles:

- BM25 scorer/reference calculations and ranked retrieval judgments;
- exact cosine top-k for ANN recall;
- straightforward interval filtering for temporal queries;
- graph/Prolog expected bindings independent of index implementation;
- hybrid relevance judgments or deterministic synthetic ground truth.

Report MRR, nDCG@k, Recall@k, ANN Recall@k, and temporal/filter correctness where applicable.

### R10. Performance and stress validation

Benchmarks must separate ingestion, embedding generation, index construction, steady-state query, mixed updates, restart/warmup, and hybrid query costs. Required evidence includes p50/p95/p99 latency, throughput, peak RSS, index/database size, write amplification where observable, candidate counts, and quality metrics.

Use deterministic local tiers up to 100K chunks: smoke at 1K–10K and full local validation at 100K. This track does not require 1M, dataset-full, external-server, or GPU benchmark tiers. Benchmarks run on the current Apple M2 Pro/10-core/32-GiB machine and must remain within its available memory and disk; first baselines establish numeric regression budgets rather than inventing unsupported thresholds. All reports record commit, machine, dataset checksums, configuration, model, dimensions, warm/cold state, and random seed.

### R11. General-knowledge validation

Validation must combine graph structure, searchable text, semantic relevance, and time—not merely run three disconnected microbenchmarks. Public license-compatible datasets may be downloaded on demand into ignored `data/benchmarks/`; source control stores only manifests, source/license attribution, checksums, deterministic conversion/sampling scripts, and compact reports. Datasets that prohibit automated download require manual placement instructions and are never redistributed. Candidate public corpora include Wikipedia/KILT or Wikidata-derived knowledge, BEIR retrieval tasks, and agent-memory datasets; final selections require license, download, checksum, ground-truth, and resource review. The approved candidate suite is introduced in batches, with every corpus/run capped at 100K chunks: (1) deterministic EnterpriseKB plus BEIR SciFact; (2) BEIR FiQA plus a license-compatible MIRACL Chinese/English subset; (3) one investigated HotpotQA/KILT multi-hop subset plus one investigated agent-memory dataset such as LoCoMo or LongMemEval. Dataset-specific use remains contingent on license, schema, checksum, and ground-truth verification. EnterpriseKB fills updates, ACL-like graph filters, intervals, revisions, multilingual fields, and deterministic-oracle gaps. ACL-style filtering is modeled through ordinary graph/Prolog predicates and verified in hybrid queries; it is not presented as mandatory security enforcement, tenant isolation, or protection against direct unfiltered index access.

### R12. Node and edge property mutation

Nodes and edges both support mutable property maps. Provide explicit set/remove/atomic-patch APIs and Prolog predicates for each entity kind. Generic `assertz/retract(property/3)` follows the approved first-version entity-resolution behavior. Set is a one-key upsert preserving other properties; remove is idempotent and `null` is not interpreted as deletion. Generic property mutation resolves an existing node or edge ID and errors on ambiguity instead of creating the wrong entity.

Expose stable edge identity lookup for Prolog joins. Edge source/type/target/ID are immutable; changing relation identity is delete-old plus create-new. Edge property updates receive source versions, table invalidation, storage/index updates, and durable index jobs exactly like node updates.

### R13. Architecture and safety

- Preserve the active `ZlfDatabase -> WamRuntime -> CompositeFactProvider` architecture.
- Keep Ollama `bge-m3:latest`/1024 dimensions as the default embedding profile while allowing versioned alternatives.
- Keep generated corpora, embeddings, index databases, and raw reports outside source control; curate only compact machine-readable evidence.
- API keys and source content must not appear in logs or committed reports.

## Non-goals for the first delivery increment

- Training embedding or reranking models.
- A distributed vector database or external search service requirement.
- GPU-only operation.
- Full-text features unrelated to measured knowledge-retrieval needs.
- Replacing Prolog/graph execution with a search-engine query language.
- Claiming semantic quality from synthetic vectors alone.

## Stage map

| Stage | Scope | Independent exit |
|---|---|---|
| 01 | Index contracts and lifecycle | versioned indexed-document identity, durable jobs, update/delete/rebuild correctness |
| 02 | BM25 correctness and scale | real BM25, bounded top-k, multilingual/update oracle and benchmarks |
| 03 | Vector/embedding correctness and scale | exact oracle, model-safe storage, ANN backend and embedding pipeline evidence |
| 04 | Temporal model and indexes | approved temporal semantics, seek-based queries, interval oracle |
| 05 | Hybrid Prolog retrieval | fusion, filters/joins, planner visibility, bounded call-time answers |
| 06 | Knowledge-base benchmark and stress | tiered public/synthetic workloads and reproducible delivery report |

See `scope-map.md` and stage requirements for dependencies.

## Parent acceptance

- All index families have versioned lifecycle-safe storage contracts and independent correctness oracles.
- Updates/deletes cannot leave silently visible stale search records beyond the documented consistency window.
- Hybrid queries combine retrieval with graph/rule/time constraints and return provenance-rich deterministic results.
- The selected scale tier meets agreed latency, throughput, quality, memory, and storage budgets.
- A fresh process can reopen indexes and reproduce correctness/quality within documented ANN tolerance.
- Full workspace quality gates pass.

## Confirmed product decisions

- Event-time plus valid-time temporal records with explicit `temporal_*` and `valid_*` predicates.
- Embedded ANN crates are allowed; exact RocksDB remains the oracle/fallback.
- Versioned pluggable embedding registry with `bge-m3` dense as default baseline.
- Durable eventual index consistency plus explicit per-index/version/timeout waits.
- Explicit chunks plus versioned built-in baseline chunkers.
- Versioned immutable `IndexProfile` declarations with explicit production fields and opt-in auto indexing.
- Mutable node/edge properties; immutable edge relation identity.
- Current M2 Pro only and at most 100K chunks per run.
- Staged public benchmark suite and graph/rule ACL-style filtering scope.
