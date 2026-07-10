# WAM Memory Roots and Lazy Storage Loading

## Current address model

WAM heap, register, environment, trail, and choice-point addresses are process-local `usize` offsets. They are never written to RocksDB. Persistent facts, rules, proofs, and table answers cross the storage boundary as typed `Term`/artifact values and are reconstructed on the active heap.

A virtual persistent heap-address layer is therefore not required for current storage scale and would create unsafe lifetime coupling. Revisit virtual addresses only if profiling shows typed term reconstruction dominates query latency.

## Lazy external relation loading

External relations now enter through the WAM call path:

```text
Call/Execute
  -> program/builtin dispatch
  -> CompositeFactProvider::facts_for_goal(current register terms)
  -> bound storage/index seek
  -> external-answer choice point
```

This removes compile-before-query materialization of every provider relation. Bound source/target/property arguments select exact, outgoing, incoming, label, or property indexes. `ZlfDatabase::explain_prolog` exposes inferred argument modes and selected access paths.

## GC/root inventory

Any future moving/compacting heap collector must treat these as roots:

1. argument and temporary registers;
2. permanent variables in environment frames;
3. saved register/environment snapshots in program and external-provider choice points;
4. trail entries until their owning choice point is discarded;
5. call/meta-call closure addresses;
6. active structure read/write cursor state;
7. builtin temporary addresses while an instruction executes;
8. proof state only where it stores heap references (current proof nodes store owned typed terms);
9. table producer/consumer state only where it stores heap references (the worklist MVP stores owned typed terms, not heap offsets).

Persistent RocksDB table answers and bulk records are not heap roots. They become roots only after decoding into active registers/heap cells.

## Bounded-memory controls

- Heap and trail are unwound to choice-point checkpoints on backtracking.
- Environments and call/cut stacks are popped on return.
- Proof state is opt-in and reset per executor run.
- `TableLimits` bounds hot table count, answers per table, and fixed-point iterations.
- Complete hot tables evict by least-recent generation and reload from RocksDB on demand.
- Provider calls materialize only the indexed bound answer set. Explicit unbound scans remain visible in query plans.

## Deferred work

- Heap compaction/generational GC implementation.
- Streaming provider cursors for answer sets too large for `Vec<Term>`.
- Persistent live SLG continuations; intentionally excluded because heap/frame offsets are process-local.
- Per-query proof-node limits if future proof workloads demonstrate a need beyond opt-in/reset semantics.
