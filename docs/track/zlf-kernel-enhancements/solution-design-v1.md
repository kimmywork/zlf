---
status: done
owner: kimmy
updated: 2026-07-10
scope_type: parent
source_requirements:
  - docs/track/zlf-kernel-enhancements/requirements-v1.md
  - docs/enhancement/roadmap.md
---

# Solution Design v1: Kernel Enhancement Roadmap Integration

## Implementation research references

Detailed implementation notes live under `docs/track/zlf-kernel-enhancements/research/`:

- `research/fact-storage-indexing.md`
- `research/builtin-predicates-and-node-view.md`
- `research/graph-algorithms.md`
- `research/tabling-and-incremental-tabling.md`
- `research/iso-prolog-compatibility.md`

These research notes define the concrete RocksDB key layouts, predicate contracts, graph algorithms, tabling/incremental tabling algorithms, and ISO/general Prolog compatibility plan that this design depends on.

## Design principles

1. Correct mutation semantics before advanced inference.
2. Stable identities before dependency tracking.
3. Deterministic tabling before incremental tabling.
4. Builtin graph algorithms before full recursive logic optimization.
5. Optional heavyweight features must not slow the default WAM path.
6. RocksDB-backed storage remains the source of durable truth; tables and proofs are derived artifacts.

## Roadmap assessment

### Worth doing soon

| Roadmap item | Decision | Rationale |
|---|---|---|
| Proof terms | Yes, after introspection metadata | Low-risk observability foundation. Needs clause/rule IDs. |
| Deterministic tabling | Yes, staged MVP | Required for cyclic recursive graph queries. |
| Incremental tabling | Yes, after base tabling | Essential for mutable persistent KB, but depends on identities/dependency graph. |
| Mode inference / predicate pushdown | Yes, after mutation and predicate registry | Needed for performance and RocksDB index use. |
| Stratified negation | Yes, after dependency graph exists | Mostly compile-time dependency analysis. |
| GC | Yes, after table/proof data exists | Required for long-running service. |
| Virtual address/lazy loading | Yes, but later | High-impact storage architecture work; not prerequisite for immediate UX gaps. |

### Defer

| Roadmap item | Decision | Rationale |
|---|---|---|
| CLP(B)/CLP(FD) | Defer | Valuable but orthogonal to graph DB correctness. |
| Probability meta-interpreter | Defer | Needs proof terms first. |
| MIL | Defer | Tooling layer, not kernel prerequisite. |
| WFS | Defer | Requires stable tabling first. |
| Higher-order logic / AC unification / linear logic | Defer | High complexity, narrow immediate value. |
| Kernel-level parallel WAM | Reject for now | Query-level multi-process is safer. |

## Dependency graph

```text
fact identity/upsert
  -> deletion/retract
  -> mutation event log
  -> dependency tracking
  -> incremental tabling

predicate registry + rule metadata
  -> rule introspection
  -> rule dependency graph
  -> proof terms
  -> stratified negation
  -> tabling declarations

node/edge convenience predicates
  -> graph algorithm builtins
  -> recursive graph workloads
  -> tabling MVP validation

base deterministic tabling
  -> table dependency graph
  -> incremental tabling
  -> WFS later
```

## Architecture slices

### Slice A: Fact identity and mutation foundation

#### Components

- `FactKey` / canonical fact identity helpers.
- Storage writer idempotent upserts.
- Indexed writer delete hooks.
- Prolog-level `retract/1` dispatcher.
- Final query result dedupe.

#### Canonical identities

| Fact form | Canonical identity |
|---|---|
| `node(Id)` | `node:{Id}` |
| `label(Id, Label)` / `Label(Id)` | `label:{Id}:{Label}` |
| `property(Id, Key, Value)` / `prop_Key(Id, Value)` | `property:{Id}:{Key}` |
| `edge(Source, Type, Target)` | `edge:{Source}:{Type}:{Target}` until explicit edge IDs are exposed |
| user rule | `rule:{predicate}:{arity}:{source_hash}` |

#### Mutation events

Every add/delete emits an internal event:

```text
FactInserted(FactKey)
FactDeleted(FactKey)
RuleInserted(RuleKey)
RuleDeleted(RuleKey)
```

Initially these events can be in-process only. Later they feed incremental tabling invalidation.

### Slice B: Predicate/rule registry and introspection

#### Components

- `PredicateRegistry` generated from builtin providers + storage shortcuts + rule store.
- Rule dependency analyzer over rule bodies.
- Provider-backed introspection predicates.

#### Predicates

```prolog
predicate(Name, Arity, Kind).
builtin_predicate(Name, Arity, Description).
rule(Name, Arity, Source).
rule_depends_on(Rule, Dependency).
```

### Slice C: Node view and graph convenience predicates

#### Components

- Storage provider support for collection/object-returning virtual predicates.
- Edge direction helpers.
- Node view serializer.

#### Predicates

```prolog
labels/2
properties/2
out_edges/2
out_edges/3
in_edges/2
in_edges/3
neighbors/2
neighbors/3
node_view/2
```

### Slice D: Proof terms

#### Components

- Clause/rule IDs in compiled artifacts.
- Optional proof stack in executor.
- Proof pointer saved in choice points.
- Query option to enable proof capture.

#### Deferred details

Proof output should initially identify facts/rules/predicates and substitutions. It does not need to store full heap snapshots.

### Slice E: Graph algorithm builtins

#### Components

- BFS/DFS algorithms over storage-backed edges.
- Bounded reachability.
- Shortest path.
- Degree counters.

#### Rationale

These deliver immediate graph value and reduce pressure to implement tabling before users can query paths safely.

### Slice F: Deterministic tabling MVP

#### Scope

- Positive predicates only.
- Variant call keys.
- Answer tables with set semantics.
- Producer state: `new`, `evaluating`, `complete`.
- Consumer support for deterministic positive recursion.
- Query-level memory table first; RocksDB cold table later.

#### Initial declaration model

Support explicit declarations before global inference:

```prolog
:- table reachable/2.
```

or internal API-level metadata if parser support is deferred.

### Slice G: Incremental tabling

#### Prerequisites

- Fact identity.
- Rule identity.
- Mutation events.
- Base tabling.
- Table dependency graph.

#### Strategy

Use invalidation-first incremental tabling before true delta maintenance:

1. Record table dependencies during evaluation.
2. On fact/rule mutation, mark dependent tables stale.
3. On next query, recompute stale tables lazily.
4. Later optimize to delta recomputation.

This is still incremental from the user's perspective and far simpler than immediate full differential fixpoint maintenance.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Direct tabling too early creates unstable complexity | Do mutation/introspection/dependency metadata first. |
| Retraction semantics conflict with graph storage | Define deletion in terms of canonical fact identities. |
| Duplicate query rows hide proof multiplicity | Default graph UX dedupe; later expose proof/all-solutions mode. |
| Recursive symmetric rules loop | Document safe base-predicate symmetric rule pattern; tabling later handles recursion. |
| Proof stack overhead | Off by default; store IDs/hashes, not full terms. |
| Incremental tabling overreach | Start with invalidation + lazy recompute, then delta. |

## Verification approach

- Unit tests for canonical fact keys and idempotent writes.
- Storage provider tests for deletion visibility.
- REPL regression for repeated facts and retraction.
- Introspection provider tests.
- Graph convenience predicate tests over RocksDB fixture.
- Graph algorithm tests over cyclic graph fixtures.
- Tabling MVP tests for cyclic reachability termination.
- Incremental tabling tests where fact deletion/insertion invalidates and refreshes reachable answers.
