---
status: proposed
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-10
version: 1
---

# Stage 06 Requirements: Knowledge-Base Benchmark and Stress

## Goal

Demonstrate that zlf works as a combined knowledge engine—not three isolated indexes—using reproducible quality, correctness, update, and scale workloads.

## Workload families

1. **Deterministic enterprise-like corpus**: documents/chunks, entities, graph relationships, validity intervals/events, multilingual text, revisions/deletes, and ACL-like graph predicates with a generated oracle.
2. **Public lexical/semantic retrieval**: select license-compatible BEIR-family tasks such as SciFact, NFCorpus, FiQA, Natural Questions, or HotpotQA according to available resources and ground truth.
3. **General knowledge graph + text**: evaluate a Wikipedia/KILT, Wikidata-derived, or similarly auditable corpus that can provide text, entity links, and multi-hop relations.
4. **Agent memory**: investigate a license-compatible temporal/long-context memory benchmark such as LoCoMo or LongMemEval; adopt only after schema and ground truth are verified.
5. **Mutation workload**: mixed inserts, revisions, deletes, embedding retries, index restart, and consistency-watermark checks.

Dataset names above are candidates, not approved dependencies, until license/checksum/resource research is recorded.

## Tiering

- smoke: 1K–10K indexed units, deterministic CI subset;
- medium: 100K;
- large: 1M;
- full: selected dataset maximum within approved machine/time/storage budget.

Each tier fixes seed, sampling, chunking, model, dimensions, analyzers, graph construction, temporal distribution, and queries.

## Metrics

- quality: MRR, nDCG@10, Recall@10/100, ANN Recall@k, hybrid deltas;
- correctness: exact expected bindings, interval/filter precision and recall, stale-result count;
- ingestion: source conversion, embedding, index build/load/update/delete throughput;
- query: p50/p95/p99, QPS, cold/warm/fresh-process, candidate/filter counts;
- resources: peak RSS, primary/index/pack sizes, queue depth/lag, CPU, and optional write amplification;
- reliability: restart recovery, stale job suppression, rebuild fallback, failed job counts.

## Acceptance

- An independent oracle validates every correctness scenario.
- Public relevance metrics are calculated from official judgments or a preserved audited conversion.
- Lexical/vector/temporal microbenchmarks and combined graph/rule/filter queries are both reported.
- At least one benchmark exercises multilingual retrieval and one exercises updates/deletes.
- Machine-readable reports include commit, dirty state, machine, dataset checksums/licenses, config, model, seed, tier, and all limits.
- Reports clearly separate embedding generation latency from retrieval latency.
- Generated data/indexes stay under ignored local directories; curated compact reports live under this track's `research/` directory.

## Open decisions

- Approved public datasets and download policy.
- Full-tier wall-time, RAM, CPU, and disk budget.
- Whether ACL filtering is a first-release requirement or a graph-filter benchmark only.
