---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-14
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-06-kb-benchmark/requirements-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-06-kb-benchmark/solution-design-v1.md
---

# Stage 06 Plan v1: Knowledge-Base Benchmark and Stress

## Dependency graph

```text
S0 report/dataset contracts
 ├──> S1 EnterpriseKB lifecycle + 100K
 ├──> S2 public dataset source/license/schema research
 │      └──> S3 FiQA + MIRACL en/zh quality
 │             └──> S4 multi-hop + memory adoption/smoke
 └──> S5 cumulative stress/reporting
S1 + S3 + S4 + S5 -> S6 review and parent-track acceptance
```

## Design choices

- **Chosen:** scripts orchestrate release Rust executables. This reuses existing crates and keeps measured behavior in production code.
- **Rejected:** a new benchmark service or query runtime; it would add technology and bypass WAM/lifecycle paths.
- **Chosen:** immutable manifests/checkpoints keyed by all data/model/index identities.
- **Rejected:** committing public/raw/generated corpora or embeddings.
- **Deferred:** ANN metrics until an ANN implementation exists; exact search is not mislabeled ANN recall.
- **Chosen:** first baselines establish performance review budgets; correctness and stale-result count are hard gates immediately.

## S0 — Contracts and harness foundation (medium)

Define dataset/run/report schemas, checksum validation, machine/commit capture, phase checkpoints, percentile/quality calculators, bounded configuration, and partial-failure reports. Migrate SciFact/EnterpriseKB H6 reports through the shared schema without changing their evidence interpretation.

Verification: schema golden tests, checksum mismatch rejection, deterministic rerun, dirty-state/machine capture, invalid/unbounded config tests.

## S1 — EnterpriseKB combined lifecycle and 100K (high)

Extend the deterministic generator and runner with revisions, deletes, inserts, embedding retry/stale jobs, process reopen, minimum-watermark waits, rebuild activation/rollback, and a 100K generated tier. Use independent graph/temporal/mutation oracles and keep embedding generation separate.

Verification: 1K smoke, 10K local regression, one 100K release run; zero stale bindings; exact mutation/delete expectations; bounded candidates/answers; restart/rebuild convergence.

## S2 — Dataset adoption research (medium)

Trace primary sources for FiQA, MIRACL en/zh, HotpotQA/KILT, LoCoMo, and LongMemEval: source URL, ownership, license/redistribution, split schema, qrels/answers, checksums, size, and required conversion. Record confidence and explicit adoption/non-adoption decisions.

Verification: every adopted dataset has a source/license/schema/checksum record and a sample parser test; unresolved licensing blocks download/redistribution, not the entire stage.

## S3 — FiQA and MIRACL multilingual quality (high)

Implement deterministic relevance-preserving adapters and <=100K subsets. Run BM25, exact `bge-m3`, and RRF on identical official qrels. Report per-language and aggregate quality, embedding time, query preparation, retrieval percentiles, candidates, RSS, and disk.

Verification: official-qrel preservation checks, all relevant documents retained, deterministic checksums, real embedding dimension/finite checks, and measured fusion deltas without assumed superiority.

## S4 — Multi-hop and agent-memory evidence (high)

Adopt one HotpotQA/KILT path and one LoCoMo/LongMemEval path only after S2. Build graph/text/temporal mappings and independent answer conversion. If evidence is insufficient, publish a sourced non-adoption report and do not fabricate a benchmark.

Verification: audited sample goldens, multi-hop graph/rule bindings, temporal-memory answer checks, bounded execution, and one local smoke report per adopted family.

## S5 — Reliability and cumulative stress (high)

Run mixed insert/revise/delete workloads, worker failure/retry, restart, generation rebuild/rollback, watermark timeout/success, and fresh-process queries. Consolidate compact machine-readable reports and compare first frozen regression budgets.

Verification: no stale results, no lost canonical mutations, expected dead/retry counts, exact active generation/watermarks, report completeness, and resource ceilings within the approved machine/tier.

## S6 — Review and delivery (medium)

Run focused and workspace gates, independently review every claim against report provenance, document limitations, accept or remediate Stage 06, then decide parent-track completion.

## Risks and rollback

- Dataset license/schema ambiguity: stop adoption and retain a sourced non-adoption decision.
- Ollama duration: checkpoint immutable embeddings; never fold embedding time into retrieval latency.
- 100K resource pressure: keep 1K/10K accepted evidence, emit partial failure metrics, optimize only the measured phase, and do not increase the approved ceiling.
- Public benchmark sampling bias: publish exact selection algorithm/checksums and avoid full-dataset leaderboard comparisons.
