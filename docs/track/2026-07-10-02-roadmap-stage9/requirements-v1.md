---
status: pending
scope_type: parent
created: 2026-07-10
version: 1
source_requirements:
  - docs/enhancement/roadmap.md
  - docs/track/zlf-kernel-enhancements/plan-v1.md
  - docs/track/zlf-kernel-enhancements/delivery-record-v1.md
---

# Roadmap Stage 9 and Advanced Runtime Requirements

## Elevator pitch

Turn the deferred Stage 9 and the still-relevant parts of the WAM enhancement roadmap into an evidence-gated program of optional logic modules and long-running runtime improvements without destabilizing the delivered WAM/storage/tabling path.

## Baseline

The predecessor track delivered kernel Stages 0–8: stable fact identity and mutation, introspection, graph providers/algorithms, ISO programming builtins, proof terms, deterministic positive tabling, persisted selective invalidation, bound storage pushdown, and mode-aware query plans.

This track must not reimplement those capabilities or claim that deterministic positive tabling is full SLG/WFS.

## Users and scenarios

- A rule author needs statically safe negation and useful diagnostics for negative recursion.
- A service operator needs bounded memory and predictable behavior during long-lived query/table/proof workloads.
- A constraint user needs opt-in Boolean and finite-domain solving that backtracks correctly.
- A knowledge engineer needs optional typed, probabilistic, or rule-learning modules without slowing ordinary queries.
- A logic researcher needs a credible path from positive tabling to WFS while preserving current persistent table semantics.

## Requirements

### R1. Stratified negation

- Build positive/negative predicate dependency edges from persisted compiled rules.
- Compute strata and reject or diagnose SCCs containing a negative dependency.
- Execute accepted stratified programs through existing WAM `\+/1` and table machinery.
- Mutations must invalidate affected analysis metadata and table results.

### R2. Mode declarations and inference

- Support explicit mode declarations and a conservative automatic inference pass.
- Preserve `+`, `-`, and unknown/bidirectional argument states.
- Surface inferred/declared modes and selected access paths through query-plan/introspection APIs.
- Never select a bound RocksDB access path unless the required argument is proven bound at that call site.

### R3. Long-running memory foundations

- Add reproducible table/proof/provider memory stress tests and memory budgets.
- Bound proof capture and large external-provider answer materialization, with explicit limit outcomes.
- Inventory and test all WAM roots before introducing a moving collector.
- Implement query arenas, provider cursors, or GC according to measured retention sources.
- Do not replace process-local heap offsets with persistent virtual pointers unless a benchmark demonstrates that typed-term reconstruction or provider loading is a dominant bottleneck.

### R4. Incremental table maintenance

- Build on persisted Stage 7 fact/rule/table dependencies.
- Distinguish insert and delete deltas and update affected positive tables without always recomputing every stale answer.
- Preserve deterministic answer order, dedupe, restart safety, and a full-recompute fallback.
- Report delta versus fallback recomputation metrics.

### R5. Optional order-sorted types

- Store type declarations and subtype DAG metadata separately from ordinary untyped terms.
- Reject subtype cycles and provide compile-time diagnostics.
- Keep the feature opt-in; untyped unification must retain its current behavior and cost profile.

### R6. Optional CLP(B) and CLP(FD)

- Use an independent trailed constraint store connected to existing variables and choice points.
- CLP(B) must support a documented Boolean expression subset and propagation.
- CLP(FD) must support finite domains, arithmetic/comparison constraints, propagation, and labeling.
- Ordinary unification and arithmetic remain unchanged when no constraint module is active.

### R7. Probabilistic proof evaluation

- Implement probability as an opt-in facade/meta layer over stable proof terms and clause metadata.
- Do not add probabilistic state to the ordinary WAM execution loop.
- Define duplicate/multiple-proof semantics explicitly before implementation.

### R8. Meta-interpretive rule learning

- Generate and validate candidate rules through the existing WAM runtime and storage facade.
- Keep learned rules pending human review; do not write them automatically to the active rule store.
- Bound candidate generation, examples, runtime, and memory.

### R9. Well-founded semantics

- Treat WFS as a separate advanced evaluator milestone after stratified negation and producer/consumer tabling prerequisites pass.
- Support true/false/undefined answers and delayed negative literals.
- Version persistent table metadata so positive tables and WFS tables cannot be confused.

### R10. Research-only capabilities

Predicate closures, AC unification, linear logic, and query-level parallelism require separate spikes and approval. They are not implementation commitments in this track's initial delivery slices.

### R11. Architecture and compatibility

- Keep `zlf-query::ZlfDatabase -> zlf-prolog::wam::WamRuntime` as the only active Prolog runtime path.
- Keep `FactProvider` read-side only and core semantics in `zlf-prolog`/WAM.
- Persist rules as compiled artifacts through `StorageRuleStore`.
- Preserve canonical lists and typed integer/float/atom/string identity.
- All optional features must be independently disabled and add no material ordinary-query overhead when disabled.

## Non-goals for the initial slice

- Full Edinburgh/ISO conformance at once.
- Full SLG/WFS in the stratified-negation slice.
- Persistent live WAM continuations or heap addresses.
- Kernel-level OR-parallel WAM execution.
- Simultaneous delivery of all optional modules.

## Parent acceptance

- Each implementation slice has an approved child requirements/design record and independent review.
- The baseline ordinary-query and full workspace gates remain green.
- New semantics have implementation-crate tests plus facade tests where user-visible.
- Long-running features have machine-readable correctness and memory/performance evidence.
- Documentation distinguishes delivered, experimental, deferred, and rejected-by-evidence capabilities.
