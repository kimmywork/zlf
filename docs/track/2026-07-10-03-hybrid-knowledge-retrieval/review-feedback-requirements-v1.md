# Review Feedback v1: Hybrid Retrieval Requirements

## Scope

Independent cumulative review of parent/stage requirements, scope map, and `research/current-state-v1.md` against the user scenarios, active architecture constraints, and requirement-discovery quality criteria.

Reviewer limitation: no separate reviewer subagent was available; this review was self-performed with claims checked against repository primary sources.

## Accuracy pass

- Current BM25, vector, temporal, embedding queue, provider, facade, tests, and benchmark claims are traceable to source files listed in the research note.
- The requirements do not claim that existing token-frequency scoring is real BM25 or that full-scan vector search is ANN.
- Existing call-time WAM/provider composition is preserved rather than replaced.
- Public dataset names are correctly marked candidates pending license/schema verification, not asserted approved dependencies.

## Findings

### F1

- **Origin phase:** requirement discovery
- **Severity:** major
- **Type:** unclear
- **Description:** First-release temporal semantics are not selected. Event time, valid time, transaction time, and bitemporal models require different identities, keys, predicates, and oracles.
- **Evidence:** parent R5/open question 1; Stage 04 is marked blocked; current implementation ignores `valid_to`.
- **Suggested fix:** product owner selects the primary model and boundary semantics before Stage 04 design.
- **Resolution:** resolved 2026-07-10 — distinct event instants plus valid-time half-open intervals `[from, to)`; `temporal_on/between` query events and `valid_at/valid_overlaps` query validity; transaction history remains internal and full bitemporal algebra is deferred.

### F2

- **Origin phase:** requirement discovery
- **Severity:** major
- **Type:** unclear
- **Description:** ANN dependency policy is unknown, preventing a credible vector implementation/scale estimate.
- **Evidence:** parent R4/open question 2 and Stage 03 open question; current exact scan is O(N).
- **Suggested fix:** confirm whether a maintained embedded ANN crate is acceptable, with exact RocksDB search retained as oracle/fallback.
- **Resolution:** resolved 2026-07-10 — embedded ANN crates are allowed; exact RocksDB remains the correctness oracle/fallback and backend selection remains benchmark-driven.

### F3

- **Origin phase:** requirement discovery
- **Severity:** major
- **Type:** unclear
- **Description:** Search consistency has no approved default. Synchronous multi-index updates and durable eventual consistency have different write latency/failure behavior.
- **Evidence:** parent R2/open question 3 and Stage 01.
- **Suggested fix:** approve durable eventual consistency with visible watermark plus optional wait-for-watermark, or require synchronous read-your-write.
- **Resolution:** resolved 2026-07-10 — durable eventual is the default; callers may wait by selected index/minimum source version/timeout, and timeout preserves primary commit while reporting pending indexes.

### F4

- **Origin phase:** requirement discovery
- **Severity:** minor
- **Type:** unclear
- **Description:** Chunk ownership was unresolved, and initial benchmark resource limits remain unresolved.
- **Evidence:** parent open questions 4–5 and Stage 06.
- **Suggested fix:** support explicit adapter chunks plus versioned built-in baseline chunkers, then discover machine budget before freezing full-tier thresholds.
- **Resolution:** resolved 2026-07-10 — hybrid chunk ownership; current M2 Pro/32-GiB machine only; smoke 1K–10K and full local validation capped at 100K chunks. Numeric regression thresholds follow first baselines.

### F5

- **Origin phase:** requirement discovery
- **Severity:** minor
- **Type:** scope
- **Description:** Six stages are appropriate but must not become one implementation batch.
- **Evidence:** scope map dependencies; lifecycle affects every backend.
- **Suggested fix:** complete/review Stage 01 contracts first; allow Stage 02/03 backend spikes in parallel only after identity/generation contracts stabilize.
- **Resolution:** already reflected in scope map; resolved.

## Coverage

| Requirement area | Coverage |
|---|---|
| users/scenarios/product roles | complete |
| current pain and source evidence | complete |
| lifecycle/update/delete/rebuild | complete |
| BM25/vector/temporal correctness | complete, temporal blocked by semantics |
| Prolog/graph hybrid composition | complete |
| quality/performance metrics | complete; numeric budgets pending environment |
| public and synthetic stress workloads | complete at candidate level |
| architecture/non-goals/security | complete |
| open decisions | explicit |

## Decision

**Pass.** All major product decisions are resolved: temporal model/predicates, ANN policy, model registry, consistency, chunk ownership, immutable index profiles and explicit field policy, node/edge property mutation, local scale, dataset policy/suite, and ACL-style filtering. Remaining backend selection, retention thresholds, fusion parameters, and numeric regression budgets are design/evidence decisions rather than unresolved product scope. Requirements may proceed to solution design; implementation still requires design review.
