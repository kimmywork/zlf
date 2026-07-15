---
status: in_progress
scope_type: standalone
created: 2026-07-15
version: 1
---

# General Prolog Fact Store

## Goal

Add a durable `FactStore` for ground Prolog facts that cannot be represented as the existing graph shortcuts, while preserving the graph-native meaning of unary labels and binary relationships.

`FactStore` is the fact counterpart to `StorageRuleStore`: it owns persisted logical facts, survives database reopen, and is exposed through the existing single WAM runtime. It is not a second Prolog engine and it does not replace RocksDB-backed nodes, edges, properties, or their indexes.

## User scenarios

- As a knowledge-base author, I can persist and query a ternary or higher-arity relation without inventing a lossy graph encoding:

  ```prolog
  impart(zlf, katurupi, tongtong).
  ? impart(zlf, From, To).
  ```

- As a graph user, I can keep using the concise graph syntax:

  ```prolog
  node(zlf).
  knows(zlf, tongtong).
  ```

  The latter remains the edge `zlf -[knows]-> tongtong`.

- As a rule author, I can use persisted logical facts as rule inputs through the same WAM execution path as graph-backed facts.

- As an operator, I can reopen a database, re-query, and retract a logical fact with deterministic identity and no duplicate answers.

## Storage-routing contract

Fact routing is syntactic and deterministic. It must not depend on which nodes, edges, or facts are currently present in a database, nor on write order.

| Prolog form | Owner | Meaning |
|---|---|---|
| Canonical `node(...)`, `label(...)`, `property(...)`, `edge(...)` forms | graph storage | Existing node, label, property, and edge contracts |
| Unreserved `P/1` | graph storage | Label shortcut: node argument has label `P` |
| Unreserved `P/2` | graph storage | Edge shortcut: first argument `-[P]->` second argument |
| Unreserved `P/N`, where `N >= 3` | `FactStore` | A first-class logical fact |

Examples:

```prolog
node(zlf).
node(tongtong).
node(katurupi).
knows(zlf, tongtong).
impart(zlf, katurupi, tongtong).
```

The first three forms create nodes, `knows/2` creates an edge, and `impart/3` is stored in `FactStore`.

## Requirements

### R1. Ground higher-arity facts

- Persist unreserved ground facts with arity three or greater in `FactStore`.
- Preserve predicate name, arity, ordered arguments, repeated arguments, nested compound terms, lists, objects, atoms/strings, and numeric values accepted by the Prolog parser.
- Reject non-ground facts explicitly; variables are not durable fact values in this increment.

### R2. Canonical fact identity

- Each stored fact has a deterministic identity based on a canonical representation of its predicate key and complete ordered term tree.
- Re-applying an identical fact is idempotent.
- Different arities, argument positions, repeated arguments, and structurally different nested terms must not collide.

### R3. Read and mutation semantics

- A `FactStore` provider resolves logical facts for matching WAM goals, including bound and partially bound arguments.
- `asserta/1`, `assertz/1`, direct REPL/CLI fact input, and `retract/1` use the same storage-routing and identity semantics.
- Logical facts remain queryable after database reopen.
- Mutation invalidates or refreshes affected runtime/table/index state through the existing write-side hooks; `FactProvider` remains read-only.

### R4. Preserve graph shortcuts

- `P/1` and `P/2` shortcut behavior remains graph-native and is covered by regression tests.
- Canonical graph predicates remain the only supported way to write graph nodes, properties, and edges outside the unary/binary shortcuts.
- Graph traversal, graph indexes, and canonical `node` / `label` / `property` / `edge` query behavior remain unchanged.

### R5. Explicit edge properties

- Property-bearing edges use only the canonical form:

  ```prolog
  edge(alice, knows, bob, { since: 2024 }).
  ```

- The former shortcut form is not retained as an edge write:

  ```prolog
  knows(alice, bob, { since: 2024 }).
  ```

  As an unreserved `P/3`, it is a logical fact under this contract, not a graph edge.
- Documentation and tests must use canonical `edge/4` for property-bearing edges.

### R6. Operational behavior

- `FactStore` uses the existing RocksDB database and follows the repository's storage-key, serialization, error, and reopen conventions.
- Provider lookup must narrow candidates by predicate key before term unification; v1 must not scan unrelated predicates for a goal.
- Retraction and write failures must not leave partially visible facts.

## Non-goals

- Arbitrary logical `P/0`, `P/1`, or `P/2` facts.
- A second Prolog evaluator, an AST-rule runtime, or runtime-only fact storage.
- Automatic reification of every fact as user-visible graph nodes/edges.
- BM25, vector, temporal, or graph traversal indexes over `FactStore` terms.
- Automatic migration of legacy `P/3` edge-shortcut source syntax. Existing graph edges remain graph edges on disk; callers must use `edge/4` going forward.

## Future plan: declared predicate properties

A later, separate track may add durable predicate declarations such as storage ownership, argument roles, indexing, and graph-view mappings. A declaration could allow an explicitly declared `P/1` or `P/2` to be a logical fact rather than a label or edge shortcut.

That future work must define declaration persistence, migration of existing shortcut predicates, conflict handling, query planning, and retraction semantics. It is intentionally deferred so this increment has one unambiguous arity-based routing contract.

## Acceptance criteria

1. `impart(zlf, katurupi, tongtong).` persists, returns correct results for exact and partially bound WAM queries, and survives reopen.
2. Distinct facts such as `a(b, b, d).` and `a(b, d, b).` remain distinct; re-inserting an identical fact produces one answer.
3. Nested ground terms and supported scalar/container terms round-trip without changing their Prolog structure.
4. `retract(impart(zlf, katurupi, tongtong)).` removes the fact and it remains absent after reopen.
5. `node(zlf).` and `knows(zlf, tongtong).` retain their current node/edge storage and query semantics.
6. `edge(alice, knows, bob, { since: 2024 }).` creates a property-bearing graph edge; `knows(alice, bob, { since: 2024 }).` does not create one.
7. Storage/provider tests cover direct writes, `asserta/assertz`, retraction, reopen, duplicate writes, malformed/non-ground facts, and no-unrelated-predicate-scan behavior.

## Planned design increments

1. Define canonical persisted term encoding, fact key/identity, RocksDB key layout, and transaction/error contract.
2. Add `FactStore` write, read, and retract operations plus focused persistence tests.
3. Route WAM/CLI dynamic fact mutation and provider queries according to the routing contract.
4. Wire invalidation, introspection, documentation, compatibility notes, and end-to-end regression coverage.
5. Produce a separate requirements/design track for declared predicate properties only when logical unary/binary facts, custom graph views, or fact indexes become necessary.

## Risks and rollback

- The `P/3` shortcut-edge behavior changes intentionally. Documentation, compiler errors/warnings where practical, and migration notes must prevent silent misuse.
- Canonical term identity is foundational for retraction and future incremental tabling; it requires independent design review before implementation.
- If provider matching cannot be bounded by predicate key without unacceptable complexity, pause before implementation rather than introducing a global fact scan.
- Rollback is achieved by not routing unreserved `P/3+` writes to `FactStore` until the new store and provider pass persistence/reopen tests.
