# Loop State: zlf

## Current Phase

Hybrid retrieval parent and Stage 01–05 child designs have passed review. Product direction changed to function-first on 2026-07-11: Tantivy BM25, exact vectors before optional hnsw_rs, ordered RocksDB temporal indexes, and bounded materialization before WAM cursors. Stage 01 I1 contracts are complete. Pre-release schema policy confirmed on 2026-07-11: no old database, migration, serialized-layout compatibility, or compatibility aliases. I0 legacy fixtures are cancelled. Stage 01 I1–I3 are complete: contracts, atomic mutation/outbox, node/edge property APIs, Prolog/JSON surfaces, edge identity, and selective invalidation. I4 durable bulk sessions/rebuild markers are complete with non-CLI workspace tests and full clippy passing. I5 immutable content-addressed profiles, JSON/Prolog lowering, deterministic chunking, and durable manifests are complete. I6 durable coordinator jobs, retry/dead/stale handling, fake target, metrics, reopen, and multi-target-safe outbox compaction are complete. Stage 01 index identity/lifecycle is delivered and accepted on 2026-07-11. Stage 02 B0–B1 are complete. Stage 02 BM25 B0–B5 is delivered and accepted on 2026-07-11: real Tantivy BM25, canonical chunk/field/language identity, bounded filters/weights/structured explanations, durable lifecycle convergence, physical generation build/validation/activation/rollback/reopen, differential oracle checks, and 1K/10K local evidence. Tantivy parameters are explicitly pinned by change note v3. Stage 03 V0 is complete with model-safe vector/job/query contracts. V1a canonical exact RocksDB storage is complete with strict ingestion, atomic batches, bounded f64 cosine/dot search, filters, ties, update/delete/reopen, and independent oracle coverage. V2 durable embedding execution and lifecycle publication are complete: immutable model registry, target-scoped manifests, outbox-driven job enqueue/delete/rebuild, bounded batch transforms, normalization/validation, exact publication, lease/retry/dead/stale handling, and fake-provider crash-safe coverage. Stage 03 vector/embedding V0–V6 is delivered and accepted on 2026-07-11: model-safe exact RocksDB vectors, durable async embedding jobs/worker, canonical lifecycle target, exact WAM/query facade, prototype removal, 1K/10K evidence, and successful Ollama OpenAI-compatible 1024-dimensional single/batch smoke gates. ANN is explicitly deferred by change note v4. Stage 04 T0 is complete: distinct event/validity contracts, UTC/date parsing, half-open semantics, ordered signed-microsecond codec, provenance, and independent boundary oracles. Stage 04 T1 is complete with generation-scoped event by-time/by-entity keys, atomic maintenance, bounded day/range/before/after/document seeks, duplicate preservation, candidate counts, generation isolation, update/delete, and reopen. Stage 04 T2 is complete with generation-scoped validity by-start/by-end/open-end/by-entity indexes, atomic maintenance, write-side endpoint estimates, bounded-memory auto-selected containment/overlap seeks, open-end merging, candidate counts, differential oracle checks, update/delete/isolation, and reopen. Stage 04 T3 is complete: profile-declared scalar/array event and validity projection, durable temporal manifests, outbox update/delete/rebuild convergence, generation-scoped runtime stores, prototype creation-date removal, approved event/valid WAM predicates, and distinct planner access paths. Stage 04 temporal semantics/indexes is delivered and accepted on 2026-07-13: explicit UTC event and half-open validity contracts, generation-scoped ordered stores, profile/outbox lifecycle, WAM predicates, planner provenance, differential oracles, and 1K/10K/100K evidence. Full workspace tests/clippy/format/size/diff gates pass. Stage 05 H0–H5 are complete: prepared hybrid retrieval now has bounded execution, filters, bound pushdown, `retrieve/4`, compact `ProofKind::Index` leaves, bound-handle/options table variants, explicit rejection of unbound/live table calls, and selective invalidation after index publication or generation changes. Preparation enforces per-target minimum published watermarks with typed timeout before WAM. H6 dataset policy is frozen: a deterministic relevance-preserving SciFact 1K-document/100-query subset with official qrels is the real lexical/vector/RRF quality gate; generated EnterpriseKB-v1 1K/10K tiers own ACL, graph/rule/time, mutation, top-k, and local scale correctness. H6a SciFact execution is complete on 2026-07-14: BM25 MRR/nDCG@10 0.816469/0.821813, exact vector 0.760906/0.782273, and RRF 0.801855/0.816503. RRF improved Recall@10/100 to 0.904667/0.990000 but did not beat BM25 MRR/nDCG, so no general fusion superiority claim is made. H6b EnterpriseKB-v1 is also complete: deterministic 1K/10K graph/property/validity fixtures produced 128/128 exact bounded filtered top-k queries, zero stale results, permission-mutation table invalidation, and 1.05/4.71 ms p99 at 1K/10K. The benchmark exposed and fixed bound canonical `property/3` full-materialization; bound entity/key goals now use direct node/edge lookup. Stage 05 H7 cumulative review passed on 2026-07-14: workspace tests, strict clippy, formatting, Rust size, and diff hygiene all pass; delivery record and acceptance review are complete. Stage 05 hybrid Prolog retrieval is accepted and delivered. Stage 06 combined knowledge-base benchmark/stress requirements, solution design, executable plan, and design review are complete on 2026-07-14. Stage 06 is in progress. S0 shared dataset/run/report contracts and harness foundation is complete: checksum-safe manifests, bounded limits, run provenance, quality/percentile helpers, identity-scoped atomic checkpoints, redacted partial failures, and the shared `zlf-benchmark-report-v1` envelope are implemented and tested. Accepted SciFact/EnterpriseKB H6 reports are migrated without losing their legacy payload. S1a EnterpriseKB generated scale is complete: deterministic 1K/10K/100K tiers now include revisions/deletes/inserts and after-mutation oracles; graph setup uses the production bulk fact-pack path; a shared-schema 100K initial-build run passed 128/128 independent oracles with zero stale results, 8.22 ms p99, approximately 783 MiB RSS, and 28.6 MiB disk. S1b mutation/restart/watermark/retry/generation lifecycle execution is next.

## Active Track

`docs/track/2026-07-10-03-hybrid-knowledge-retrieval/`

Goal: productionize BM25, vector/embedding, and temporal indexes; compose them with WAM graph/rule queries; validate quality, lifecycle correctness, and scale on general knowledge-base workloads.

## Pending Track

`docs/track/2026-07-10-02-roadmap-stage9/`

Deferred by product decision on 2026-07-10. Do not begin stratified negation, CLP, WFS, probability, MIL, or advanced runtime work until explicitly resumed.

## Delivered Baseline

- Kernel enhancement Stages 0–8 are complete.
- Canonical storage mutation, graph providers/algorithms, ISO core builtins, proof terms, deterministic positive tabling, persistent selective invalidation, bound storage pushdown, and query planning are available.
- NCBI Taxonomy bulk/scale track is complete.

## Confirmed Hybrid Retrieval Decisions

1. Event time plus valid-time half-open intervals; `temporal_*` and `valid_*` remain distinct.
2. Embedded ANN crates allowed; exact RocksDB oracle/fallback retained.
3. Pluggable model registry with `bge-m3` dense baseline.
4. Durable eventual consistency with per-index/version/timeout wait.
5. Explicit chunks plus versioned built-in chunkers.
6. Immutable versioned IndexProfiles through Prolog directive and JSON/Rust APIs.
7. Mutable node/edge properties; immutable edge relation identity.
8. Current M2 Pro only, at most 100K chunks per run.
9. Staged EnterpriseKB/BEIR/multilingual/multi-hop/agent-memory benchmark suite.

## Stage 05 Delivered Evidence

Real quality dataset:

```bash
python3 scripts/prepare-scifact.py \
  --output data/benchmarks/scifact \
  --documents 1000 \
  --queries 100 \
  --seed zlf-h6-scifact-v1
```

Expected readiness marker:

```text
data/benchmarks/scifact/h6-1000d-100q-v1/manifest.json
```

The subset must include all positive-qrel documents for the deterministically selected queries, then hash-ranked distractors. BM25, exact `bge-m3`, and RRF `k=60` must use the identical corpus/query/qrels files. Embedding build time is reported separately from retrieval latency. Quality claims use official judgments only.

Preparation details and verification commands:

```text
docs/track/2026-07-10-03-hybrid-knowledge-retrieval/
  2026-07-10-05-hybrid-prolog/research/dataset-preparation-v1.md
```

EnterpriseKB-v1 is generated, not downloaded. The next H6 implementation increment owns its deterministic generator and 1K/10K tiers; do not manually author it. It validates ACL-style ordinary graph/rule filtering, event/validity constraints, permission mutations, stale-result behavior, candidate selectivity, top-k ordering, latency, RSS, and disk. It must not be presented as real semantic quality evidence.

The SciFact manifest was prepared and checksum-validated. H6a benchmark implementation and local execution are recorded in `implementation-progress-v7.md` and `research/scifact-h6-local-2026-07-14.{json,md}`. EnterpriseKB-v1 generation and 1K/10K execution are complete in `implementation-progress-v8.md` and `research/enterprise-kb-h6-*`. H6 deliberately separates real-quality SciFact evidence from generated-oracle EnterpriseKB composition evidence. H7 accepted Stage 05 in `review-feedback-implementation-v1.md` and `delivery-record-v1.md`. Stage 06 requirements are confirmed and its design/plan passed review in `2026-07-10-06-kb-benchmark/`. S0 is recorded in `2026-07-10-06-kb-benchmark/implementation-progress-v1.md` and `benchmark-contract-v1.md`. S1a is recorded in `implementation-progress-v2.md` and `research/enterprise-kb-s1-100k-2026-07-14.{json,md}`. The next active increment is S1b: apply and independently verify generated insert/revise/delete operations, reopen, embedding retry/stale jobs, minimum-watermark waits, and generation rebuild activation/rollback. The accepted 100K initial-build report explicitly does not claim those remaining lifecycle checks.

## Local Exclusions

`.agents/prompt-history/`, `data/`, generated corpora, embeddings, indexes, and raw benchmark outputs remain untracked unless a compact curated report is intentionally added under a track's `research/` folder. `/data/benchmarks/` is explicitly ignored.
