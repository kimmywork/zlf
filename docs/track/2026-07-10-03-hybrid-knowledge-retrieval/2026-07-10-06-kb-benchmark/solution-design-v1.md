---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-14
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-06-kb-benchmark/requirements-v1.md
---

# Stage 06 Solution Design v1: Knowledge-Base Benchmark and Stress

## Scope

Stage 06 turns the focused Stage 02–05 fixtures into one reproducible benchmark suite. It does not add another retrieval runtime, index backend, query language, or security layer. The maximum local tier remains 100K indexed chunks on the current M2 Pro/32 GiB machine.

## Benchmark contract

Every run consumes a versioned dataset manifest and emits one JSON report with:

```text
schema, commit, dirty state, machine, dataset name/version/license/source/checksums,
seed, tier, profiles/chunking/analyzers/model/dimension, generations/watermarks,
all candidate/page/answer limits, phase timings, query percentiles, quality,
correctness, candidates/selectivity, RSS/disk, failures/retries/stale counts
```

Phase timing separates conversion, canonical ingestion, index publication, document embedding, query embedding, warm retrieval, and fresh-process retrieval. “Fresh process/reader” is not labeled OS-cold unless filesystem caches are independently controlled.

Generated corpora, embeddings, databases, and raw output stay under ignored `data/benchmarks/`. Git stores deterministic adapters/generators, manifests without corpus payloads, attribution/license notes, checksums, compact reports, and commands.

## Dataset adapters

Adapters lower source data into one audited intermediate form:

```text
Document { source_id, text fields, language, entities, revision }
Query { query_id, text, user/context/time, judgments }
GraphFact / TemporalRecord / Mutation
```

Public qrels remain unchanged. Deterministic subset selection first keeps all judged-positive documents, then hash-samples distractors. Missing public judgments are not replaced with handcrafted labels. Embedding caches are keyed by dataset checksum, transformed text fingerprint, immutable model profile/version/revision, and dimension.

## Workload sequence

1. **EnterpriseKB + SciFact foundation:** reuse the accepted H6 generators/reports, add combined lifecycle restart, revisions, deletes, embedding retries, minimum-watermark waits, and 100K generated stress.
2. **FiQA + MIRACL English/Chinese:** adopt only after source URL, license, split/schema, qrels, and checksum are recorded. Use deterministic <=100K subsets and real `bge-m3` embeddings for multilingual quality.
3. **Multi-hop + memory:** investigate HotpotQA versus KILT and LoCoMo versus LongMemEval. Adopt one from each family only when redistribution/license, schema, answer/judgment conversion, and graph/temporal mapping are confirmed. Otherwise publish a documented non-adoption decision rather than weak metrics.

## Execution architecture

A Python orchestration layer prepares data and invokes release Rust benchmark executables. Rust owns canonical ingestion, index lifecycle, WAM queries, correctness comparisons, and process metrics. The harness persists phase checkpoints so document embeddings and completed indexes can be reused only when their full identity matches.

Enterprise mutation phases execute:

```text
initial build -> warm/fresh query -> insert/revise/delete batch -> worker retry/restart
-> minimum-watermark wait -> query/oracle comparison -> generation rebuild/activation/rollback
```

Every correctness scenario compares bindings/order against an independent adapter oracle. Retrieval quality uses standard MRR, nDCG@10, and Recall@10/100 over preserved public qrels. ANN Recall@k is omitted while ANN remains deferred, not reported as exact-vector Recall@k.

## Resource and failure policy

All work is bounded by documents/chunks, queries, batch size, candidates, pages, answers, retries, and timeout. A failed phase emits a partial report with error category and last checkpoint. Secrets and source text are excluded from errors and compact reports.

Numeric regression budgets are established from first clean baselines rather than guessed in advance. Correctness, stale-result count, finite limits, and report completeness are hard gates; latency/RSS/disk changes initially remain measured review gates.

## Acceptance

- EnterpriseKB covers combined graph/rule/retrieval/temporal lifecycle, updates/deletes, restart, and stale-result oracle checks at 1K/10K and one 100K generated tier.
- At least one public official-qrel benchmark and one English/Chinese multilingual benchmark run with real embeddings.
- Multi-hop and memory candidates receive sourced adoption/non-adoption records; adopted datasets receive executable smoke evidence.
- Reports preserve provenance and separate synthetic correctness, protocol smoke, and real relevance claims.
- Workspace quality gates and deterministic preparation checks pass.
