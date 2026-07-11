# Loop State: zlf

## Current Phase

Hybrid retrieval parent and Stage 01–05 child designs have passed review. Product direction changed to function-first on 2026-07-11: Tantivy BM25, exact vectors before optional hnsw_rs, ordered RocksDB temporal indexes, and bounded materialization before WAM cursors. Stage 01 I1 contracts are complete. Pre-release schema policy confirmed on 2026-07-11: no old database, migration, serialized-layout compatibility, or compatibility aliases. I0 legacy fixtures are cancelled; continue directly with I2 atomic mutation/outbox.

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
