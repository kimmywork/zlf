---
status: in_progress
scope_type: parent
created: 2026-07-10
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/requirements-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/solution-design-v1.md
---

# Plan v1: Hybrid Knowledge Retrieval

## Dependency graph

```text
P0 contracts/tests
 -> P1 property mutation + storage outbox
 -> P2 profiles/chunks/generations/workers
      ├-> P3 BM25 backend
      ├-> P4 vector/model/ANN backend
      └-> P5 temporal backend
           \    |    /
            -> P6 paged WAM providers + hybrid retrieval
            -> P7 datasets, quality, stress, delivery
```

No backend stage may invent a separate document identity, job queue, or generation lifecycle.

## P0 — Freeze contracts and baseline failures

**Risk:** low

- Add compact checked-in fixtures that expose current stale BM25 update/delete, vector model/dimension overwrite, temporal `valid_to` omission, and edge property update gap.
- Freeze serializable `EntityRef`, `PropertyPatch`, `IndexDocumentId`, `IndexProfileArtifact`, `EmbeddingModelProfile`, generation/status, mutation event, retrieval request/hit, and metrics contracts.
- Record current 1K/10K prototype baseline without claiming BM25/ANN correctness.
- Verify old database open/migration requirements before changing `Edge` serialization.

**Exit:** contracts compile, fixtures fail for the intended current limitations, and migration strategy is reviewed.

## P1 — Node/edge property mutation and durable outbox

**Risk:** high

- Add atomic storage property patch operations for nodes and edges.
- Add external node/edge indexing version/tombstone metadata and `edge_id/4` lookup without changing the existing serialized `Edge` shape.
- Implement explicit Rust/JSON/Prolog set/remove operations and repair generic property dynamic writes.
- Allocate mutation sequences and atomically publish index-agnostic outbox records with primary record plans.
- Cover API, Prolog, import, retract, delete/cascade, replay, restart, and bulk rebuild-required marker.
- Connect table invalidation to node/edge property mutation without weakening Stage 7 selectivity.

**Exit:** all entity mutations are correct without any search backend; every supported write has exactly one replay-safe source event or explicit bulk marker.

## P2 — Profiles, chunks, generations, and coordinator

**Risk:** high

- Implement profile/model/chunk artifacts and validators in `zlf-index`.
- Persist immutable profiles and activation history; add Prolog directive and JSON/Rust APIs through one lowering path.
- Implement explicit/whole/paragraph/fixed-window chunk extraction and per-entity manifests.
- Implement worker claim/retry/dead-letter/stale suppression and contiguous per-target watermarks.
- Implement fresh-generation build, validation, activation, rollback, reopen, and bounded retention.
- Add status, wait-for-version, rebuild, and metrics APIs/CLI.

**Exit:** a deterministic fake index target proves lifecycle behavior under crashes, stale jobs, profile changes, rebuild failure, and restart.

## P3 — BM25 correctness and scale

**Risk:** medium

- Implement one common lexical backend contract.
- Spike a mature embedded backend and custom RocksDB alternative on the same 10K fixture; record analyzer/lifecycle/performance evidence.
- Select one backend through design change note/review.
- Implement real field-aware BM25, Jieba-compatible versioned analysis, replace/delete, top-k, tie-break, explanations, generation/reopen.
- Differential-test hand-calculated corpus and independent scorer.
- Run 1K/10K/100K lexical quality and performance tiers.

**Exit:** update-safe BM25 quality/latency/resource report passes; prototype token-count schema is rejected/rebuilt explicitly.

## P4 — Embedding registry, exact vectors, and ANN

**Risk:** high

- Implement versioned model registry and separate query/document transforms.
- Refactor durable embedding jobs for batch, lease, retry/backoff, dead letter, stale suppression, and redacted diagnostics.
- Implement multi-document/model canonical vector storage with strict validation and exact top-k oracle.
- Benchmark embedded ANN candidates, including an annembed/HNSW-class candidate, at 10K/100K.
- Select backend by Recall@k, latency, build/reopen/update, RSS, disk, license, and maintenance evidence.
- Implement derived ANN generation, tombstone/rebuild policy, fallback, and fresh-process validation.
- Run deterministic vector tests and opt-in local `bge-m3` quality/throughput tiers.

**Exit:** exact and ANN results meet agreed Recall@k; incompatible model/dimension/generation never mix; failed embedding does not lose primary data.

## P5 — Event-time and valid-time indexes

**Risk:** medium/high

- Implement ordered UTC instant encoding and separate event/validity records.
- Implement event time/entity indexes and half-open date/range predicates.
- Implement validity start/end/entity indexes and containment/overlap intersection.
- Add profile extraction, updates/deletes, generations, cursor/page queries, and planner access-path reporting.
- Differential-test all boundaries and skew/open intervals.
- Run 1K/10K/100K temporal performance tiers and decide from evidence whether coarse buckets are needed.

**Exit:** no normal bound temporal predicate scans the complete index; oracle and restart/update tests pass.

## P6 — Hybrid facade and WAM composition

**Risk:** high

- Implement structured async retrieval request/context and prepared query embedding.
- Add `retrieve/4` while retaining existing index predicates.
- Implement RRF baseline, deterministic tie-break, provenance/explanation, and minimum-watermark waits.
- Add provider cursor/page support to external WAM choice points with cut/backtracking/proof tests.
- Implement filter-first and progressive retrieval-first graph/ACL plans and query-plan visibility.
- Add index generation/watermark table dependencies and worker-completion invalidation.
- Test graph/property/rule/temporal joins, multiple goal orders, top-k/filter exhaustion, limits, mutation, restart, proof, and table behavior.

**Exit:** bounded hybrid queries return deterministic provenance-rich results and never call remote embedding from WAM.

## P7 — General knowledge-base benchmark and delivery

**Risk:** medium

- Build deterministic EnterpriseKB generator and independent graph/time/ACL/relevance oracle.
- Add manifest/download/conversion for approved batch 1, then batch 2 and batch 3 after license/schema review.
- Build one runner that separates conversion, load, embedding, build, cold/warm query, mixed mutation, and quality evaluation.
- Run 1K–10K smoke and at most 100K local tiers on the M2 Pro.
- Record source/model/profile/checksum/seed/machine, p50/p95/p99/QPS, RSS/disk, jobs/watermarks, MRR/nDCG/Recall, ANN Recall, stale count, and hybrid deltas.
- Freeze regression budgets from accepted baseline evidence.
- Complete implementation review, delivery review, delivery record, and user acceptance.

**Exit:** combined text/vector/time/graph/rule workloads pass independent oracles and accepted local quality/performance budgets.

## Acceptance mapping

| Parent requirement | Plan |
|---|---|
| R1 identity/profile/chunks | P0/P2 |
| R2 lifecycle/consistency | P1/P2 |
| R3 BM25 | P3 |
| R4 vectors/models/ANN | P4 |
| R5 temporal | P5 |
| R6 hybrid composition | P6 |
| R7 bounded call-time execution | P5/P6 |
| R8 observability | P2–P7 |
| R9 correctness/quality | P3–P7 |
| R10 local stress | P3–P7 |
| R11 benchmark suite | P7 |
| R12 node/edge property mutation | P1 |
| R13 architecture/safety | all |

## Verification gates per increment

Focused implementation tests run first. Before each child delivery:

```bash
cargo fmt --all -- --check
python3 scripts/check-rust-size.py
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
git diff --check
```

Opt-in provider/public-data tests record environment and may not replace deterministic CI oracles.

## Stop/rollback conditions

- Return to requirements if a backend requires an external mandatory service, exceeds the 100K local scope, changes temporal semantics, or weakens explicit field/privacy controls.
- Return to design if edge migration cannot open old databases, outbox cannot be atomic with primary writes, selected backend cannot rebuild by generation, ANN quality misses the approved threshold, or cursor state breaks WAM backtracking/cut.
- Fall back to exact vectors, prior active generations, full index rebuild, or existing predicates rather than ship incorrect/stale retrieval.

## Immediate next action after design approval

Start P0/P1 only. Do not implement BM25/ANN/temporal replacements until the entity mutation, outbox, profile, and generation contracts pass their child design and implementation reviews.
