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
- **Resolution:** roll-back to user clarification; open.

### F2

- **Origin phase:** requirement discovery
- **Severity:** major
- **Type:** unclear
- **Description:** ANN dependency policy is unknown, preventing a credible vector implementation/scale estimate.
- **Evidence:** parent R4/open question 2 and Stage 03 open question; current exact scan is O(N).
- **Suggested fix:** confirm whether a maintained embedded ANN crate is acceptable, with exact RocksDB search retained as oracle/fallback.
- **Resolution:** user clarification; open.

### F3

- **Origin phase:** requirement discovery
- **Severity:** major
- **Type:** unclear
- **Description:** Search consistency has no approved default. Synchronous multi-index updates and durable eventual consistency have different write latency/failure behavior.
- **Evidence:** parent R2/open question 3 and Stage 01.
- **Suggested fix:** approve durable eventual consistency with visible watermark plus optional wait-for-watermark, or require synchronous read-your-write.
- **Resolution:** user clarification; open.

### F4

- **Origin phase:** requirement discovery
- **Severity:** minor
- **Type:** unclear
- **Description:** Chunk ownership and initial benchmark resource limits are unresolved.
- **Evidence:** parent open questions 4–5 and Stage 06.
- **Suggested fix:** use adapter-owned deterministic chunking initially and discover machine budget before freezing full-tier thresholds, unless product requirements differ.
- **Resolution:** may use recommended defaults after major decisions; open.

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

**Requirements are well-shaped but not yet approved for solution design.** Resolve F1–F3 with the product owner. F4 can use the documented recommended defaults unless overridden. Do not begin implementation.
