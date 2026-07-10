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
6. **ACL-style graph filtering**: users/groups/classifications/grants and Prolog access rules filter hybrid candidates; this validates composition, not a complete security subsystem.

The approved candidate sequence is: (1) EnterpriseKB + SciFact; (2) FiQA + a MIRACL Chinese/English subset; (3) one HotpotQA/KILT multi-hop subset + one investigated agent-memory dataset such as LoCoMo or LongMemEval. Dataset-specific adoption remains contingent on license/checksum/schema/ground-truth research. License-compatible data may be downloaded on demand into ignored `data/benchmarks/`; repository artifacts are limited to manifests, attribution/license notes, checksums, deterministic conversion/sampling scripts, and compact reports. Non-redistributable datasets require manual placement instructions.

## Tiering

All benchmarks run on the current Apple M2 Pro/10-core/32-GiB development machine:

- smoke/CI: 1K–10K indexed chunks;
- full local: at most 100K indexed chunks.

No 1M, dataset-full, external-server, or GPU tier is required by this track. Public datasets larger than 100K chunks are deterministically sampled. Each tier fixes seed, sampling, chunking, model, dimensions, analyzers, graph construction, temporal distribution, and queries. Initial runs establish numeric wall-time/RSS/disk regression budgets while recording the machine's available resources.

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

- Final choice between HotpotQA/KILT and between agent-memory candidates after license/schema investigation; the staged candidate suite and download policy are confirmed.
- Numeric regression thresholds after the first M2 Pro baselines; machine and maximum 100K-chunk scope are confirmed.
- Specific public benchmark suite; ACL scope is confirmed as graph/rule filtering only.
