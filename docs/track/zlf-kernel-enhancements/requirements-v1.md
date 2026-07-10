---
status: done
owner: kimmy
updated: 2026-07-10
scope_type: parent
source_requirements:
  - docs/enhancement/roadmap.md
  - docs/track/zlf/requirements-v1.md
  - docs/track/zlf/plan-v2.md
---

# Requirements v1: zlf WAM Kernel Enhancements

## Goal

Plan the next production-readiness increment for zlf as a WAM-backed Prolog graph database, combining the enhancement roadmap with recently discovered product gaps around fact identity, deletion, introspection, graph querying, tabling, and incremental tabling.

## Core conclusion

The roadmap is directionally worthwhile, especially deterministic tabling and incremental tabling. However, incremental tabling should not be implemented first. It depends on stable fact identity, mutation semantics, rule dependency metadata, table dependency tracking, and base tabling correctness.

## Users and scenarios

### U1. Interactive graph user

- Adds facts repeatedly from REPL or import scripts.
- Expects duplicate facts not to produce duplicate query rows.
- Needs to delete nodes, labels, properties, and edges.
- Needs simple queries for a node's labels, properties, edges, and neighbors.

### U2. Rule author

- Defines derived graph predicates.
- Needs to list stored rules and builtin predicates.
- Needs safe ways to model undirected edges without infinite recursion.
- Needs explanation/proof output when a rule result is surprising.

### U3. Large knowledge-base operator

- Runs recursive graph queries such as reachability over cyclic graphs.
- Needs termination and deduplicated answers.
- Needs fact updates to invalidate or refresh derived results incrementally.
- Needs predictable memory behavior for long-lived service processes.

## Required capabilities

### R1. Fact identity and idempotent writes

- Every persisted fact-like write must have a canonical identity.
- Re-applying the same fact must be idempotent.
- Query results should dedupe identical final binding maps by default for graph-database UX.

### R2. Retraction and deletion

Support deletion through explicit API and Prolog-level mutation forms:

- `retract(node(Id)).`
- `retract(label(Id, Label)).`
- `retract(Label(Id)).`
- `retract(property(Id, Key, _)).`
- `retract(prop_Key(Id, _)).`
- `retract(edge(Source, Type, Target)).`
- `retract(EdgeType(Source, Target)).`

Deletion must update storage, provider-visible facts, indexes, embedding queues, and future table invalidation metadata.

### R3. Predicate and rule introspection

Expose predicates for:

- user rules
- builtin predicates
- provider-backed predicates
- shortcut predicates
- rule dependencies

Minimum forms:

```prolog
predicate(Name, Arity, Kind).
builtin_predicate(Name, Arity, Description).
rule(Name, Arity, Source).
rule_depends_on(Rule, Dependency).
```

### R4. Node view and graph convenience predicates

Expose common graph views without requiring users to manually assemble multiple predicates:

```prolog
labels(Node, Labels).
properties(Node, Properties).
out_edges(Node, Edges).
out_edges(Node, Type, Edges).
in_edges(Node, Edges).
in_edges(Node, Type, Edges).
neighbors(Node, Neighbor).
neighbors(Node, Type, Neighbor).
node_view(Node, View).
```

### R5. Safe undirected edge modeling

- Document and support derived symmetric predicates over directed stored edges.
- Avoid direct self-recursive definitions such as `friend(X, Y) :- friend(Y, X).` without base predicates.
- Later schema may support `edge_type(Type, { directed: false })`.

### R6. Proof terms and traceability

Support optional proof output for successful answers. Proof capture must be off by default and cheap when disabled.

### R7. Deterministic tabling MVP

Support positive recursive predicates with set semantics and termination over cyclic graphs.

MVP scope:

- variant call table
- answer table
- answer dedupe
- producer/consumer state for deterministic positive recursion
- no negation, aggregation, or WFS in MVP

### R8. Incremental tabling

Support dependency-aware table invalidation after fact/rule mutation.

Minimum scope:

- fact identity to table dependency edges
- table to table dependency edges
- invalidation propagation
- lazy recompute on next query
- persistence of dependency metadata where practical

### R9. Graph algorithm builtins before full Datalog

Before full tabling is complete, provide high-value graph builtins implemented in Rust:

```prolog
reachable(Source, Target).
reachable(Source, Target, MaxDepth).
shortest_path(Source, Target, Path).
degree(Node, Degree).
in_degree(Node, Degree).
out_degree(Node, Degree).
```

### R10. ISO/general Prolog programming capabilities

Support the common programming capabilities expected from general Prolog systems, staged after mutation foundations and before full tabling:

- canonical ISO list representation with `[H|T]` pattern matching;
- arithmetic evaluator and arithmetic predicates;
- string/atom/chars/codes conversion subset;
- type tests and term decomposition predicates;
- control predicates and meta-call subset;
- ISO-style dynamic database predicates mapped to zlf storage/rule store;
- practical standard-library subset, especially `library(lists)`.

### R11. Production runtime foundations

The roadmap items below are valid but should follow correctness and mutation foundations:

- mode inference and predicate pushdown
- virtual address/lazy fact loading
- GC for long-lived tables and proof data
- stratified negation after dependency graph exists

## Non-goals for the next immediate increment

- Full WFS.
- Full CLP(FD)/CLP(B).
- Probabilistic logic.
- MIL/rule learning.
- High-order logic.
- Kernel-level parallel WAM execution.

## Acceptance criteria

- Duplicate fact writes are idempotent and covered by tests.
- Retraction/deletion works for node, label, property, and edge facts.
- Introspection predicates expose builtin/provider/user-rule metadata.
- Node view and graph convenience predicates work against RocksDB-backed storage.
- A safe undirected edge recipe is documented and tested.
- Track plan identifies tabling and incremental tabling prerequisites clearly.
- Future tabling work has explicit MVP boundaries and verification criteria.
