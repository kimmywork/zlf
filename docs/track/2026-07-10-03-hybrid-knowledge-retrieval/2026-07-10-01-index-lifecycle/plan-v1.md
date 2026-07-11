---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-01-index-lifecycle/requirements-v1.md
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-01-index-lifecycle/solution-design-v1.md
---

# Stage 01 Plan v1: Index Identity and Lifecycle

## Goal

Deliver parent P0–P2 as reviewed, independently verifiable increments, starting with first-version contracts and ending with a fake-target lifecycle proof. Production BM25/vector/temporal replacement remains blocked.

## Dependency graph

```text
I1 shared contracts
 -> I2 atomic mutation planner + outbox
      -> I3 property/edge APIs + WAM/JSON
      -> I4 bulk/reopen coverage
I1 -> I5 profiles + chunking
I2 + I5 -> I6 coordinator + fake target
I6 -> I7 generations/status/wait/retention
I3 + I4 + I7 -> I8 cumulative review and stage evidence
```

## I1 — Freeze shared contracts

**Risk:** medium

- Add `EntityRef` and `PropertyPatch` to `zlf-core`.
- Add mutation event/state/receipt contracts to `zlf-storage`.
- Split `zlf-index` contract modules by identity, profile, model, generation, retrieval, and metrics to satisfy source-size policy.
- Add canonical binary key codecs, schema versions, serde snapshots, validation, and deterministic fingerprints.

**Verification:** unit/snapshot/property tests for round trips, ordering, malformed values, patch conflicts, dimensions/model metadata, and user IDs containing separators.

**Exit:** all required P0 contracts compile and serialize deterministically for the first-version schema.

## I2 — Atomic canonical mutation and outbox

**Risk:** high

- Add the storage write mutex and lifecycle metadata bootstrap.
- Implement `MutationPlan`, sequence-range allocation, entity state/tombstones, and ordered outbox reads.
- Refactor create/update/delete, labels, edge deletion, and cascade paths to one batch each.
- Preserve node version history and current graph index behavior.
- Add idempotent patch no-op semantics while preserving committed full-replacement behavior, plus stale-event state checks.
- Add the storage-neutral opaque projection-config commit used to order profile activation atomically with the outbox.

**Verification:** storage integration matrix, concurrent sequence test, cascade event ordering, new-schema reopen, and atomic failure injection before commit.

**Exit:** every effective canonical entity mutation commits primary/index/state/outbox records atomically; no-op operations emit no event.

## I3 — Property mutation and public surfaces

**Risk:** high

- Implement node/edge atomic patches and set/remove convenience methods.
- Add entity ambiguity detection and ordered edge identity lookup.
- Update fact lowering/retract behavior and add explicit WAM predicates.
- Add `ZlfDatabase` and JSON-over-STDIO requests/responses in focused modules.
- Connect changed predicates to existing selective table invalidation.

**Verification:** Rust, WAM, query-facade, and CLI integration tests for set preservation, remove idempotence, `null`, edge mutation, ambiguity, parallel edges, relation immutability, assert/retract, and tables.

**Exit:** node and edge properties have equivalent lifecycle semantics across supported APIs.

## I4 — Imports, bulk sessions, and mutation audit

**Risk:** medium/high

- Audit every graph write call site and classify it as canonical, bulk, or unsupported raw metadata.
- Keep ordinary JSON import on canonical methods.
- Add durable bulk session/checkpoint/finalization and one rebuild marker.
- Add reopen handling for incomplete sessions and update `zlf-bulk` integration.
- Document raw storage methods as non-graph APIs.

**Verification:** import event counts, interrupted/resumed bulk load, final marker uniqueness, restart, and a checked-in write-path matrix.

**Exit:** all supported graph write paths produce entity events or one explicit bulk rebuild marker; no silent bypass remains.

## I5 — Profiles, models, documents, and chunkers

**Risk:** medium/high

- Implement one artifact validator/store and activation history through the primary-storage projection-config commit.
- Implement field matcher/options and required analyzer/model/temporal/key identities.
- Implement explicit, whole-field, paragraph/heading, and fixed-window chunks.
- Add deterministic document IDs/fingerprints/source ranges and entity/profile manifests.
- Parse JSON/Rust first; add Prolog directive lowering through the same store.

**Verification:** golden chunks for English/Chinese/mixed text, Unicode byte ranges, overlap boundaries, immutable artifact conflicts, activation sequence, manifest replacement/deletion, reopen.

**Exit:** deterministic extraction produces stable, non-colliding document identities and one persisted profile artifact path.

## I6 — Coordinator and fake target

**Risk:** high

- Add per-target job/lease/retry/dead/stale state and contiguous scanned/published watermarks.
- Implement ordered event expansion against profile activation history.
- Build an idempotent durable fake target with deterministic failure injection.
- Implement stale suppression against current entity state and crash recovery around target write/ack boundaries.
- Add outbox compaction gated by all active target watermarks.

**Verification:** state-machine/property tests, stale update/delete replay, expired leases, bounded retries, permanent failures, irrelevant events, out-of-order completion, process reopen, and crash matrix.

**Exit:** fake target converges exactly to canonical documents and cannot resurrect stale content.

## I7 — Generations, status, waits, and retention

**Risk:** high

- Implement draft/build/checkpoint/validate/activate/retire/fail transitions.
- Build fake generations from a captured storage sequence and reconcile later events.
- Atomically switch active pointers only after validation.
- Add status, inventory, metrics, wait, rebuild/resume, and timeout results through Rust and JSON/CLI.
- Enforce active+previous, failed-generation, and dead-letter retention defaults.

**Verification:** invalid transitions, failed validation rollback, interrupted resume, active pointer atomicity, fresh-process read, wait success/timeout, pending target report, retention safety.

**Exit:** lifecycle acceptance passes against the fake target and prior active generations survive failed rebuilds.

## I8 — Cumulative verification and review

**Risk:** medium

- Run focused tests after each increment.
- Audit acceptance and write-path coverage.
- Run implementation review across requirements, design, code, and tests; resolve all critical/major findings.
- Record compact evidence and update loop state.

**Required gates:**

```bash
cargo fmt --all -- --check
python3 scripts/check-rust-size.py
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
git diff --check
```

**Exit:** Stage 01 acceptance is evidenced; only then may backend child designs consume these contracts.

## Acceptance mapping

| Stage acceptance | Increments |
|---|---|
| exact live docs after insert/update/remove/delete/cascade/replay | I2, I3, I6 |
| stale job cannot resurrect content | I2, I6 |
| crash between target write and acknowledgment recovers | I6 |
| rebuild failure preserves active generation | I7 |
| reopen preserves jobs/generations/watermarks | I4, I6, I7 |
| all write paths tested or explicitly unsupported | I3, I4, I8 |
| profiles/chunks/version metadata | I1, I5 |
| status/wait/metrics/retention | I7 |

## Stop and rollback conditions

- Return to design if one-batch canonical mutation cannot include sequence/outbox state or bulk finalization cannot expose interrupted sessions.
- Do not add a `zlf-index` dependency to `zlf-storage` to work around event design.
- Do not enable a production backend consumer until fake-target stale/crash/generation tests pass.
- Disable target activation and retain graph/outbox state if coordinator behavior is incorrect; never roll back a committed primary mutation because indexing failed.
