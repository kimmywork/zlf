# Loop State: zlf

## Current Phase

Hybrid retrieval parent and Stage 01–05 child designs have passed review. Product direction changed to function-first on 2026-07-11: Tantivy BM25, exact vectors before optional hnsw_rs, ordered RocksDB temporal indexes, and bounded materialization before WAM cursors. Stage 01 I1 contracts are complete. Pre-release schema policy confirmed on 2026-07-11: no old database, migration, serialized-layout compatibility, or compatibility aliases. I0 legacy fixtures are cancelled. Stage 01 I1–I3 are complete: contracts, atomic mutation/outbox, node/edge property APIs, Prolog/JSON surfaces, edge identity, and selective invalidation. I4 durable bulk sessions/rebuild markers are complete with non-CLI workspace tests and full clippy passing. I5 immutable content-addressed profiles, JSON/Prolog lowering, deterministic chunking, and durable manifests are complete. I6 durable coordinator jobs, retry/dead/stale handling, fake target, metrics, reopen, and multi-target-safe outbox compaction are complete. Stage 01 index identity/lifecycle is delivered and accepted on 2026-07-11. Stage 02 B0–B1 are complete. Stage 02 BM25 B0–B5 is delivered and accepted on 2026-07-11: real Tantivy BM25, canonical chunk/field/language identity, bounded filters/weights/structured explanations, durable lifecycle convergence, physical generation build/validation/activation/rollback/reopen, differential oracle checks, and 1K/10K local evidence. Tantivy parameters are explicitly pinned by change note v3. Stage 03 V0 is complete with model-safe vector/job/query contracts. V1a canonical exact RocksDB storage is complete with strict ingestion, atomic batches, bounded f64 cosine/dot search, filters, ties, update/delete/reopen, and independent oracle coverage. V2 durable embedding execution and lifecycle publication are complete: immutable model registry, target-scoped manifests, outbox-driven job enqueue/delete/rebuild, bounded batch transforms, normalization/validation, exact publication, lease/retry/dead/stale handling, and fake-provider crash-safe coverage. Stage 03 vector/embedding V0–V6 is delivered and accepted on 2026-07-11: model-safe exact RocksDB vectors, durable async embedding jobs/worker, canonical lifecycle target, exact WAM/query facade, prototype removal, 1K/10K evidence, and successful Ollama OpenAI-compatible 1024-dimensional single/batch smoke gates. ANN is explicitly deferred by change note v4. Stage 04 T0 is complete: distinct event/validity contracts, UTC/date parsing, half-open semantics, ordered signed-microsecond codec, provenance, and independent boundary oracles. Stage 04 T1 is complete with generation-scoped event by-time/by-entity keys, atomic maintenance, bounded day/range/before/after/document seeks, duplicate preservation, candidate counts, generation isolation, update/delete, and reopen. Stage 04 T2 is complete with generation-scoped validity by-start/by-end/open-end/by-entity indexes, atomic maintenance, write-side endpoint estimates, bounded-memory auto-selected containment/overlap seeks, open-end merging, candidate counts, differential oracle checks, update/delete/isolation, and reopen. Stage 04 T3 is complete: profile-declared scalar/array event and validity projection, durable temporal manifests, outbox update/delete/rebuild convergence, generation-scoped runtime stores, prototype creation-date removal, approved event/valid WAM predicates, and distinct planner access paths. Stage 04 T4 is complete with atomic temporal batches and differential 1K/10K/100K uniform, skewed, and long-open evidence. Worst 100K p99 is below 65 ms, RSS 531.8 MB, disk below 97 MB per distribution; evidence does not justify buckets/interval trees. Continue with T5 cumulative review and fresh workspace acceptance. Parent hybrid retrieval remains in progress.

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

## Local Exclusions

`.agents/prompt-history/`, `data/`, generated corpora, embeddings, indexes, and raw benchmark outputs remain untracked unless a compact curated report is intentionally added under a track's `research/` folder.
