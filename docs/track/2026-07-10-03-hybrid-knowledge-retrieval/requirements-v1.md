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

### R1. Unified indexed-document identity

Every indexed unit must have stable identity independent of a physical index key. The minimum logical identity is:

```text
IndexDocumentId = node_id + field + chunk_id
```

Index records must carry source version/content fingerprint, analyzer or embedding model identity, and index schema version. Multiple fields, chunks, languages, embedding models, and historical validity records must not overwrite one another accidentally.

### R2. Lifecycle correctness

Node/fact API writes, Prolog dynamic writes/retracts, imports, bulk loads, updates, and deletes must produce a durable index mutation stream or an explicit "indexes not updated" outcome. Indexing may be eventually consistent, but jobs must be idempotent, retryable, observable, and version-checked so an old job cannot overwrite a newer document version.

The system must support index rebuild, validation, checkpoint/resume, and generation-based atomic publication. Search must never silently mix incompatible analyzer/model/schema generations.

### R3. Real lexical retrieval

BM25 must use document frequency, corpus/document length statistics, configurable `k1`/`b`, deterministic tie-breaking, field/chunk identity, bounded top-k, and update/delete correctness. Chinese and English analysis behavior must be versioned and tested. Phrase/position search and advanced analyzers are optional unless benchmark evidence requires them.

### R4. Scalable vector retrieval

Vector indexing must validate dimensions and model identity, support text-query and source-node-query workflows, provide exact search as a correctness oracle, and add a pluggable approximate nearest-neighbor backend selected by benchmark evidence. Search contracts must include top-k, threshold, model/index generation, deterministic tie-breaking, and whether the source item is included.

Embedding generation throughput, failure/retry behavior, batching, dedupe, stale-job suppression, and provider/model metadata must be measured separately from vector retrieval latency.

### R5. Explicit temporal semantics

Temporal indexing must distinguish the chosen domain semantics—event time, validity interval, transaction time, or bitemporal data—and define interval boundaries precisely. Point, overlap/range, before, and after queries must use ordered index seeks rather than full scans at target scale. Mutation and deletion must remove superseded temporal records.

The exact first-delivery temporal model is an open product decision.

### R6. Hybrid retrieval and Prolog composition

The facade must support:

- lexical-only, vector-only, and temporal-only retrieval;
- hybrid lexical/vector fusion with an explicit algorithm such as reciprocal-rank fusion;
- graph, property, label, rule, and temporal filters;
- retrieval-first and bound-entity lookup plans;
- score, rank, source field/chunk, model/analyzer generation, and optional explanation metadata.

Raw BM25 and cosine scores must not be added directly without calibration. Existing predicates remain compatible; new option-bearing APIs/predicates require an explicit contract and planner visibility.

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

Use deterministic tiers before full scale, for example 10K, 100K, 1M, and dataset-full documents/chunks. All reports record commit, machine, dataset checksums, configuration, model, dimensions, warm/cold state, and random seed.

### R11. General-knowledge validation

Validation must combine graph structure, searchable text, semantic relevance, and time—not merely run three disconnected microbenchmarks. Candidate public corpora include Wikipedia/KILT or Wikidata-derived knowledge, BEIR retrieval tasks, and agent-memory datasets; final selections require license, download, checksum, ground-truth, and resource review. A deterministic enterprise-like synthetic corpus fills coverage gaps such as updates, ACL-like graph filters, and interval edge cases.

### R12. Compatibility and safety

- Preserve the active `ZlfDatabase -> WamRuntime -> CompositeFactProvider` architecture.
- Keep default embedding configuration compatible with Ollama `bge-m3:latest`/1024 dimensions while allowing versioned alternatives.
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

## Open questions

1. Which temporal model is primary for the first release: valid-time intervals, event timestamps, transaction history, or bitemporal?
2. May zlf add an embedded ANN library, or must the first scalable vector backend be implemented only with RocksDB/current dependencies?
3. Should index consistency be synchronous for API writes, or is durable eventual consistency with a visible watermark acceptable by default?
4. Which public datasets may be downloaded and retained locally, and what machine/runtime/storage budget should define the full tier?
5. Is document chunking owned by zlf or supplied by ingestion adapters in the first release?
6. Are ACL/security filters required in the first enterprise benchmark or only modeled as ordinary graph predicates?
