---
status: proposed
scope_type: parent
created: 2026-07-10
version: 1
source_requirements:
  - docs/track/2026-07-10-02-roadmap-stage9/requirements-v1.md
  - docs/track/2026-07-10-02-roadmap-stage9/scope-map.md
---

# Solution Design v1: Advanced Logic and Long-Running Runtime

## Design stance

The roadmap is an architectural input, not a mandate to implement every mechanism literally. The delivered runtime already avoids whole-relation loading through typed call-time providers and bound RocksDB seeks. Consequently, persistent 64-bit heap pointers and a moving collector are evidence-gated rather than assumed prerequisites.

The track is contract-first and sliceable. Every optional module must use the existing parser, typed `Term`, WAM heap/unifier/trail/choice points, compiled rule artifacts, providers, and `ZlfDatabase` facade.

## A. Signed dependency and stratification layer

Extend rule metadata with signed edges:

```text
RuleClauseId -> Dependency {
  predicate: PredicateKey,
  polarity: Positive | Negative,
}
```

Compilation computes predicate SCCs. An SCC containing a negative internal edge is non-stratified. The first slice returns a structured compile/storage error and does not persist the invalid artifact. The condensation DAG receives integer strata; negative edges must target a lower stratum.

Accepted rules continue to execute `\+/1` through the existing WAM path. Static analysis supplies the semantic guarantee; it does not introduce a second evaluator. Registry/rule mutation refreshes signed dependencies and selectively invalidates dependent tables.

## B. Mode system and provider cursors

Represent modes independently from runtime values:

```text
ArgumentMode = In | Out | Unknown
PredicateMode = { predicate, arguments, origin: Declared | Inferred }
```

Inference is conservative and monotone. Constants and previously bound variables provide `In`; successful relational goals may establish outputs only when their declared/inferred contract guarantees it. Conflicting clauses degrade to `Unknown`, never to an unsafe `In`.

`QueryPlan` exposes declaration origin, before/after binding sets, and selected storage access. Provider APIs gain cursor/page contracts for potentially large answer streams while preserving external-answer choice-point behavior. Existing `facts_for_goal -> Vec<Term>` remains as a compatibility adapter until providers migrate.

## C. Memory lifecycle

Start with observability and ownership boundaries:

- per-query heap/trail/environment high-water marks;
- proof-node and provider-answer counts;
- hot/cold table bytes and eviction counts;
- process RSS sampled by stress runners.

Apply the least invasive remedy to the measured source:

1. query-owned arenas reset at query completion;
2. explicit proof/provider/table limits and typed limit errors;
3. streaming provider cursors to avoid large vectors;
4. mark/compact or generational collection only for demonstrated reachable long-lived heap objects.

Any collector uses the root inventory in the predecessor track and runs only at safe points. Persistent terms remain serialized values. A virtual pointer layer requires a separate accepted spike proving material benefit over provider seeks and typed decoding.

## D. Delta table maintenance

Persist mutation epochs and delta records separately from complete table answers:

```text
MutationDelta { epoch, kind: Insert | Delete, fact_or_rule_key }
TableDeltaState { base_generation, applied_epoch, status }
```

The Stage 7 reverse dependency indexes identify affected tables. For inserts, seed affected rules with new facts and propagate newly derived answers. For deletes, maintain support counts/provenance sufficient to remove only unsupported answers; if support data is absent, limits are exceeded, or a rule shape is unsupported, mark stale and use the existing full fixed-point recomputation.

Publish answers, dependency metadata, support metadata, and applied epoch atomically. Interrupted delta evaluation loads as stale. Cyclic propagation uses a visited worklist and deterministic answer ordering.

## E. Constraint modules

Constraint state is query-local and opt-in:

```text
ConstraintStore {
  variable_attributes,
  propagator_queue,
  trail_checkpoint,
}
```

Variable binding invokes module hooks only for attributed variables. Every domain narrowing, propagator activation, and queue mutation is trailed. CLP(B) establishes the hook and propagation contracts first. CLP(FD) then adds interval/domain representation, arithmetic propagators, and labeling choice points. Stored graph properties remain ordinary typed values unless explicitly posted into a constraint domain.

## F. Order-sorted type module

Type declarations compile to a versioned subtype DAG and optional predicate signatures. Cycle checks and rule diagnostics happen before artifact publication. Runtime typed matching is enabled only for predicates/modules declaring the feature. This avoids changing canonical storage fact identity or ordinary unification globally.

## G. Proof-driven meta modules

Probability and MIL remain outside the core instruction loop.

- Probability metadata attaches to stable fact/rule clause IDs. The proof facade enumerates proofs and applies an explicitly selected aggregation model. Initial support is restricted to finite proof sets and documented independence assumptions.
- MIL consumes bounded positive/negative examples, metarules, and background predicate allowlists. It invokes ordinary WAM queries for validation and emits candidate artifacts into a review queue, never directly into `StorageRuleStore`.

## H. WFS extension

WFS is not implemented as an enhancement to `\+/1`. It requires fuller SLG producer/consumer suspension, delayed literals, unfounded-set handling, and three-valued answers. Introduce a separately versioned table kind:

```text
PositiveComplete | WfsComplete
Truth = True | False | Undefined
```

Positive table readers must reject WFS metadata and vice versa. Persistent live continuations remain out of scope; interrupted evaluation restarts stale.

## Public contracts

Planned API families, finalized by child designs:

- `ZlfDatabase::analyze_rules` / structured stratification diagnostics;
- mode declaration/introspection and enriched `explain_prolog`;
- query/proof/table resource limits and metrics;
- incremental maintenance metrics and fallback reason;
- opt-in constraint/type/probability/MIL facades;
- three-valued WFS query results distinct from ordinary binding maps.

No facade may assemble a reduced provider/runtime environment that changes ordinary query semantics.

## Verification strategy

Each slice requires:

1. implementation-crate unit/integration tests;
2. `zlf-query` facade tests for user-visible behavior;
3. persistence/restart and mutation tests where metadata is durable;
4. differential oracle tests for constraints, delta tables, or WFS;
5. disabled-feature regression benchmarks;
6. workspace format, size, clippy, and test gates.

Memory and scale claims require machine-readable reports recording dataset, commit, limits, wall time, peak RSS, table metrics, and fallback counts.

## Key risks and mitigations

- **Semantic overclaiming:** name each evaluator scope explicitly; never label positive worklist tabling as full SLG/WFS.
- **Unsafe mode inference:** degrade uncertainty to `Unknown`; validate plans against actual bindings.
- **Delete-delta complexity:** support counts plus mandatory full-recompute fallback.
- **Constraint contamination:** query-local attributed-variable hooks disabled by default.
- **Proof explosion:** limits and finite-proof requirements before probability aggregation.
- **GC corruption:** safe-point collector only after root tests and stress reproduction.
- **Track sprawl:** optional slices require child approval and can be closed independently.
