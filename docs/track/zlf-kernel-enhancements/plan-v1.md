---
status: in_progress
owner: kimmy
updated: 2026-07-08
scope_type: parent
source_requirements:
  - docs/track/zlf-kernel-enhancements/requirements-v1.md
  - docs/track/zlf-kernel-enhancements/solution-design-v1.md
---

# Plan v1: Kernel Enhancement Implementation Track

## Implementation research references

Use these implementation notes while executing the stages:

- `research/fact-storage-indexing.md`: canonical fact identity, RocksDB graph indexes, deletion, mutation events.
- `research/builtin-predicates-and-node-view.md`: exact builtin/provider predicate contracts and node view shapes.
- `research/graph-algorithms.md`: neighbors, degree, reachability, shortest path algorithms on RocksDB indexes.
- `research/tabling-and-incremental-tabling.md`: deterministic tabling MVP, dependency tracking, invalidation, lazy recompute, and delta roadmap.

## Goal

Deliver the next production-readiness layer for zlf's WAM-backed Prolog graph database: stable fact mutation semantics, graph/query introspection, graph convenience predicates, proof metadata, graph algorithm builtins, deterministic tabling, and eventually incremental tabling.

## Priority summary

The roadmap's incremental tabling item is worthwhile and should remain a target, but it is not the next first task. The implementation should proceed through dependency-ordered stages:

```text
Stage 0: Fact identity + mutation semantics
Stage 1: Introspection + predicate registry
Stage 2: Node view + graph convenience predicates
Stage 3: Graph algorithm builtins
Stage 4: Proof terms / traceability
Stage 5: Deterministic tabling MVP
Stage 6: Incremental tabling invalidation
Stage 7: Storage/performance foundations
Stage 8: Optional advanced logic modules
```

## Stage 0: Fact identity, idempotent writes, and deletion

**Status:** todo  
**Risk:** medium  
**Why first:** Incremental tabling cannot be correct without stable fact identities and mutation events.

### Tasks

- Add canonical fact key helpers for node/label/property/edge/rule facts.
- Make `StorageFactWriter` and `IndexedStorageFactWriter` idempotent for repeated fact writes.
- Add final query binding dedupe in WAM runtime or `ZlfDatabase` facade.
- Add storage deletion methods if missing:
  - delete node with incident edge cleanup
  - delete edge by id/triple
  - remove label
  - delete property
- Add Prolog-level `retract/1` dispatcher for supported fact forms.
- Add JSON-over-STDIO/HTTP command coverage for delete/remove operations.
- Emit internal mutation events for add/delete operations.

### Acceptance

- Repeating `person(alice).` does not create duplicate `person(alice)` answers.
- Repeating `edge(alice, knows, bob).` does not create duplicate edges.
- `retract(person(alice)).` removes the label shortcut result.
- `retract(prop_name(alice, _)).` removes the property.
- `retract(edge(alice, knows, bob)).` removes both `edge/3` and `knows/2` visibility.
- Node deletion removes incident edges and index entries.

### Verification

```bash
cargo test -p zlf-prolog storage_fact_identity
cargo test -p zlf-query repl_retract_regression
cargo test -p zlf-cli
python3 scripts/check-rust-size.py
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
```

## Stage 1: Predicate registry and introspection

**Status:** todo  
**Risk:** medium

### Tasks

- Add `PredicateRegistry` for builtin/provider/rule predicates.
- Add rule dependency analyzer over `PrologRule` bodies.
- Persist rule source hash / rule key if not already enough.
- Expose provider-backed introspection predicates:
  - `predicate/3`
  - `builtin_predicate/3`
  - `rule/3`
  - `rule_depends_on/2`
- Include dynamic shortcut predicates where discoverable:
  - labels as `label_shortcut`
  - edge types as `edge_shortcut`
  - properties as `property_shortcut`

### Acceptance

- `? predicate(Name, Arity, Kind).` lists builtins and user rules.
- `? rule(friend, 2, Source).` returns persisted rule source.
- `? builtin_predicate(edge, 3, Description).` succeeds.
- `? rule_depends_on(friend/2, knows/2).` succeeds for `friend(X,Y) :- knows(X,Y).`

## Stage 2: Node view and graph convenience predicates

**Status:** todo  
**Risk:** low-medium

### Tasks

- Add storage provider predicates:
  - `labels/2`
  - `properties/2`
  - `out_edges/2`, `out_edges/3`
  - `in_edges/2`, `in_edges/3`
  - `neighbors/2`, `neighbors/3`
  - `node_view/2`
- Define JSON/Object term representation for edge lists and node views.
- Document safe undirected edge modeling:

```prolog
friend(X, Y) :- friend_edge(X, Y).
friend(X, Y) :- friend_edge(Y, X).
```

### Acceptance

- `? labels(alice, Labels).` returns all labels as a list.
- `? properties(alice, Props).` returns object-like properties.
- `? out_edges(alice, Edges).` returns outgoing edge objects.
- `? neighbors(alice, Neighbor).` enumerates adjacent nodes.
- `? node_view(alice, View).` returns labels/properties/in/out edges.

## Stage 3: Graph algorithm builtins

**Status:** todo  
**Risk:** medium

### Tasks

- Add Rust-backed graph builtins over storage/provider:
  - `reachable/2`
  - `reachable/3`
  - `shortest_path/3`
  - `degree/2`
  - `in_degree/2`
  - `out_degree/2`
- Use bounded BFS for `reachable/3` and `shortest_path/3`.
- Add cycle-safe graph fixtures.

### Acceptance

- Cyclic graphs terminate.
- `reachable(alice, X, 3)` respects max depth.
- `shortest_path(alice, carol, Path)` returns a shortest path list.

## Stage 4: Proof terms and traceability

**Status:** todo  
**Risk:** medium-high

### Tasks

- Add stable clause IDs for facts/rules/provider facts where practical.
- Add optional proof stack to WAM executor.
- Save proof pointer in choice points.
- Add query option/API to request proof output.
- Return compact proof JSON referencing rule/fact IDs and substitutions.

### Acceptance

- Normal query path has no proof overhead beyond disabled branch checks.
- Proof-enabled query returns answer + proof tree.
- Backtracking rolls proof stack back correctly.

## Stage 5: Deterministic tabling MVP

**Status:** todo  
**Risk:** high

### Tasks

- Add table declaration metadata, initially explicit:

```prolog
:- table reachable/2.
```

- Add variant call canonicalization.
- Add in-memory table store:
  - key
  - state: evaluating/complete/stale
  - answer set
  - consumers
- Add table-aware call path for tabled predicates.
- Start with positive recursion only.
- Add answer dedupe.

### Acceptance

- Recursive reachability over cyclic graph terminates.
- Repeated subgoals reuse table answers.
- Non-tabled predicates keep existing WAM behavior.

## Stage 6: Incremental tabling invalidation

**Status:** todo  
**Risk:** high

### Why after Stage 5

Incremental tabling requires base table correctness plus dependencies from facts/rules to tables. Implementing it before stable fact identity and tabling would create invalidation metadata with no reliable target.

### Tasks

- During table evaluation, record dependencies:
  - table -> fact keys
  - table -> table keys
  - table -> rule keys
- Persist dependency metadata where feasible.
- On mutation event, mark affected tables stale.
- On next query, lazily recompute stale tables.
- Later optimize stale recomputation to delta maintenance.

### Acceptance

- Insert/delete of an edge invalidates `reachable/2` tables depending on that edge.
- Next query returns refreshed results without full process restart.
- Unrelated tables remain valid.

## Stage 7: Storage and performance foundations

**Status:** todo  
**Risk:** high

### Tasks

- Mode inference / mode declarations.
- Predicate pushdown to storage provider and RocksDB indexes.
- Persistent predicate key layouts for bound argument seeks.
- Virtual address/lazy loading design spike.
- GC roots inventory for heap/environment/choice/trail/table/proof data.

### Acceptance

- Bound first-arg storage predicates avoid full scans.
- Query plans expose pushed-down constraints.
- Memory growth is bounded in long-running table/proof scenarios.

## Stage 8: Optional advanced logic modules

**Status:** deferred

Candidate modules:

- stratified negation
- CLP(B)/CLP(FD)
- probabilistic meta-interpreter
- MIL/rule learning
- WFS

These should not start until Stages 0-6 are stable.

## Immediate next implementation recommendation

Start Stage 0 and Stage 1 together only where they share metadata:

1. Implement canonical fact keys and idempotent writes.
2. Add deletion/retract support.
3. Add query result dedupe.
4. Add predicate/rule introspection registry.
5. Add tests for repeated facts and retraction in REPL/StorageFactWriter.

This solves the most visible product issues and creates the identity/dependency substrate needed by incremental tabling.

## Quality gates

Each stage must pass:

```bash
cargo fmt --all -- --check
python3 scripts/check-rust-size.py
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
```

Large/ignored integration tests remain opt-in:

```bash
cargo test -p zlf-prolog --test wiki_full_pipeline -- --ignored --nocapture
cargo test -p zlf-prolog --test ollama_embedding_provider -- --ignored --nocapture
```
