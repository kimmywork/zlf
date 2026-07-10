# Review Feedback v1: Hybrid Retrieval Solution Design

## Scope

Cumulative review of approved requirements, current-state investigation, stage requirements, `solution-design-v1.md`, and `plan-v1.md`.

Reviewer limitation: no independent reviewer subagent was available; this is a separate self-review pass checked against repository source and architecture constraints.

## Accuracy and consistency

- The design preserves the active WAM/provider/storage path and does not introduce a second evaluator or write-capable `FactProvider`.
- Prototype limitations and delivered kernel capabilities match the cited source investigation.
- Product decisions are represented consistently across parent and stage documents.
- Profile/model/index generations are derived projections; graph storage remains source of truth.
- The plan caps local workloads at 100K chunks and does not retain obsolete 1M/full acceptance language.
- Event and validity predicates remain distinct and use approved half-open semantics.

## Findings and resolutions

### D1

- **Origin phase:** solution design
- **Severity:** major
- **Type:** correctness
- **Description:** Adding a `version` field directly to serialized `Edge` would risk opening old bincode records.
- **Evidence:** current `Edge` is serialized directly; bincode records have no explicit envelope migration in the current path.
- **Suggested fix:** keep indexing source version/tombstone in external metadata and use the mutation sequence; leave `Edge` shape unchanged in the initial slice.
- **Resolution:** fixed in design and P1 plan.

### D2

- **Origin phase:** solution design
- **Severity:** major
- **Type:** correctness
- **Description:** An outbox written only by `ZlfDatabase` would miss direct storage and WAM dynamic mutation paths.
- **Evidence:** current WAM dynamic builtins call `StorageFactWriter`/`Storage` directly.
- **Suggested fix:** place index-agnostic mutation sequence/outbox publication in atomic `zlf-storage` mutation batches, below all facades.
- **Resolution:** already designed and made a P1 exit criterion.

### D3

- **Origin phase:** solution design
- **Severity:** major
- **Type:** scope
- **Description:** Selecting BM25 and ANN crates without focused evidence could create technology sprawl or lock in weak lifecycle behavior.
- **Evidence:** requirements allow crates but require correctness/rebuild/100K evidence.
- **Suggested fix:** preserve common contracts; run bounded candidate spikes; select through a reviewed change note before production backend implementation.
- **Resolution:** incorporated in P3/P4; no backend is prematurely selected.

### D4

- **Origin phase:** solution design
- **Severity:** major
- **Type:** correctness
- **Description:** Applying top-k before graph/ACL filtering can return too few results or omit authorized high-ranked results.
- **Evidence:** retrieval and Prolog filtering are separate operations in the current provider model.
- **Suggested fix:** support filter-first where selective and progressive retrieval-first paging/over-fetch until accepted top-k or exhaustion; report guarantee/exhaustion.
- **Resolution:** incorporated in hybrid design and P6 acceptance.

### D5

- **Origin phase:** solution design
- **Severity:** major
- **Type:** feasibility
- **Description:** Cursor-backed provider answers must survive WAM backtracking and be discarded correctly by cut.
- **Evidence:** current external choice points store materialized answers.
- **Suggested fix:** require a P6 child design/test matrix before changing the provider trait; keep compatibility adapter and explicit limits.
- **Resolution:** planned as high-risk P6 work and not authorized by parent review alone.

### D6

- **Origin phase:** solution design
- **Severity:** minor
- **Type:** performance
- **Description:** Dual valid-time endpoint indexes can still scan large skewed candidate sets.
- **Evidence:** long/open intervals can make one endpoint side broad.
- **Suggested fix:** report scanned candidates at 1K/10K/100K and add buckets/another interval structure only if the approved local distribution violates budgets.
- **Resolution:** present in P5; accepted as evidence-gated design detail.

### D7

- **Origin phase:** solution design
- **Severity:** minor
- **Type:** operability
- **Description:** Retention counts and numeric regression thresholds are not fixed.
- **Evidence:** product owner approved current-machine baseline-first policy.
- **Suggested fix:** retain active + previous generation initially and freeze configurable retention/performance limits after measured baselines.
- **Resolution:** design and plan state this policy.

## Acceptance coverage

| Area | Design coverage |
|---|---|
| node/edge property mutation | explicit patches, generic compatibility, immutable relation identity |
| atomic lifecycle | storage-level sequence/outbox, stale suppression, watermarks |
| profiles/chunks | immutable artifacts, dual API, deterministic chunking, manifests |
| BM25 | backend comparison, real scoring, oracle, replacement/delete |
| vectors/models | registry, batching, exact oracle, ANN generation/fallback |
| temporal | event/validity indexes and approved predicates |
| Prolog hybrid | prepared embedding, retrieve relation, RRF, filters, proof/table deps |
| operations | generations, rebuild, rollback, status/wait/metrics |
| benchmark | approved datasets, independent oracles, 100K local reports |
| rollback | prior generation, exact fallback, durable outbox, schema isolation |

## Decision

**Pass for parent design and implementation planning.** No unresolved critical or major findings remain in the parent artifacts. Implementation must still proceed by reviewed child slices: begin P0/P1 only; P3–P6 require their focused backend/cursor designs and evidence before selection or delivery claims.
