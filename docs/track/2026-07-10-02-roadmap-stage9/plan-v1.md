---
status: pending
scope_type: parent
created: 2026-07-10
version: 1
source_requirements:
  - docs/track/2026-07-10-02-roadmap-stage9/requirements-v1.md
  - docs/track/2026-07-10-02-roadmap-stage9/solution-design-v1.md
---

# Plan v1: Roadmap Stage 9 and Advanced Runtime

## Execution policy

This is a parent plan, not authorization to implement every optional module. Open a child track for each slice, freeze its contracts and oracle, obtain independent design review, then implement. Do not run S5–S10 as one monolithic change.

## S0 — Baseline and contracts

**Status:** pending

- Curate positive/negative recursion and mode-analysis corpora.
- Add long-running memory/performance harness and machine-readable report schema.
- Record ordinary-query disabled-feature baselines.
- Define feature availability/introspection conventions.

**Exit:** reproducible semantic and memory baselines exist; no roadmap mechanism is selected without evidence.

## S1 — Stratified negation

**Status:** planned

- Persist signed rule dependencies.
- Compute SCCs, strata, and negative-cycle diagnostics.
- Reject non-stratified artifacts atomically.
- Test rule mutation, restart, table invalidation, and accepted NAF behavior.

**Exit:** accepted programs are stratified and negative recursion is never silently assigned two-valued semantics.

## S2 — Modes and storage cursors

**Status:** planned

- Define mode declaration syntax/artifact metadata.
- Implement conservative inter-clause inference.
- Enrich query plans with mode origin and binding transitions.
- Add cursor/page provider API and migrate storage scans.

**Exit:** every selected bound index is justified by a proven input; large result providers can run within a configured memory bound.

## S3 — Memory lifecycle

**Status:** planned

- Instrument WAM/table/proof/provider memory.
- Add proof and provider limits with typed outcomes.
- Stress query-owned reclamation and hot-table eviction.
- Implement arenas or GC only against reproduced retention.
- Run 7x24-hour soak only after shorter deterministic tiers pass.

**Exit:** agreed workload remains within its budget and no unresolved monotonic growth is observed.

## S4 — Delta incremental tables

**Status:** planned

- Define mutation epochs, support metadata, and atomic publication.
- Implement insert delta for supported positive rules.
- Implement delete support removal for supported rules.
- Preserve full stale recomputation fallback and expose reasons.
- Verify cycles, restart, duplicate derivations, and NCBI mutation workloads.

**Exit:** supported updates produce oracle-equivalent answers with less work than full recomputation; unsupported cases remain correct through fallback.

## S5 — Order-sorted types

**Status:** optional

- Child requirements must define syntax, storage metadata, and unification boundary.
- Implement DAG validation and compile diagnostics before runtime matching.

## S6 — CLP(B)

**Status:** optional

- Child requirements must define operators, reification, propagation, and labeling scope.
- Establish trailed attributed-variable/constraint-store contracts.

## S7 — CLP(FD)

**Status:** optional; depends on S6 infrastructure review

- Define finite domain representation and supported constraints.
- Add propagation and deterministic labeling options with an independent oracle.

## S8 — Probabilistic proof facade

**Status:** optional

- Define probability metadata and multi-proof semantics first.
- Enforce proof/count/time limits and keep ordinary WAM execution unchanged.

## S9 — MIL review tool

**Status:** optional; depends on S8/proof-limit contracts

- Define metarule language, examples, hypothesis bounds, scoring, and review workflow.
- Emit candidates only; require explicit promotion to persisted rules.

## S10 — WFS

**Status:** optional/high risk; depends on S1 and fuller tabling design

- Produce a dedicated SLG suspension/delay design.
- Introduce separately versioned three-valued table storage.
- Differential-test against a trusted WFS implementation.

## S11 — Research spikes

**Status:** deferred

- Predicate closures beyond existing `call/N`.
- AC unification and linear logic.
- Query-level process/thread deployment model.
- Persistent virtual addresses only if S2/S3 evidence justifies them.

Research findings do not enter the active runtime without a new approved implementation slice.

## Global quality gates

```bash
cargo fmt --all -- --check
python3 scripts/check-rust-size.py
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
git diff --check
```

Ignored Ollama/wiki and long-running soak/stress suites remain opt-in and must record their environment.

## Immediate next action

Start only S0/S1 requirement discovery: define signed dependency semantics, accepted/rejected examples, directive syntax, diagnostic shape, and mutation/restart acceptance tests. S2 may perform read-only investigation in parallel but must not change provider contracts before its child design is approved.
