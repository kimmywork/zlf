# Stage 04 Implementation Progress v5

## Increment T4 — Differential local scale evidence

**Status:** completed on 2026-07-11.

### Delivered

- Atomic event and validity batch replace/delete APIs; validity statistics refresh once per affected generation rather than once per record.
- Reproducible release benchmark at 1K, 10K, and 100K records per kind over uniform, duplicate/skewed, and long-open distributions.
- Every measured query is differentially asserted against the independent event/containment/overlap oracle.
- Reports include build/update throughput, matches, candidates, p50/p95/p99, RSS, and disk.
- Evidence-backed decision not to add buckets/interval trees: selective paths remain bounded and worst 100K p99 is below 65 ms; deliberate all-match scans cannot avoid returning the full result population.

### 100K evidence

- Build 200K event+validity records: 377.9–434.2 ms.
- Selective event p99: 0.06–0.44 ms; deliberate all-match skew event p99: 33.41 ms.
- Valid-at p99: 0.60–27.68 ms.
- Overlap p99: 13.80–61.93 ms.
- Peak process RSS: 531.8 MB; per-distribution disk: 94.1–96.6 MB.
- Replace 200 records: 25.3–29.5 ms.

Raw JSON tiers, methodology, interpretation, and frozen budgets are under `research/temporal-local-*2026-07-11*`.

### Next

T5 performs cumulative review and fresh workspace acceptance, then marks Stage 04 delivered if all gates pass.
