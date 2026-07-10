# Loop State: zlf

## Current Phase

Requirement discovery and current-state investigation.

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

## Immediate Discovery Decisions

1. First-release temporal model.
2. Embedded ANN dependency policy.
3. Synchronous versus durable eventual index consistency.
4. Chunk ownership boundary.
5. Public benchmark datasets and full-tier resource budget.

## Local Exclusions

`.agents/prompt-history/`, `data/`, generated corpora, embeddings, indexes, and raw benchmark outputs remain untracked unless a compact curated report is intentionally added under a track's `research/` folder.
