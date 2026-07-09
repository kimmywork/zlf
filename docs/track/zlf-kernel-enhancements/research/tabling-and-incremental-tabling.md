# Deterministic Tabling and Incremental Tabling

## 1. Purpose

This note defines a concrete implementation path for zlf tabling. The goal is to support recursive graph/knowledge queries over cyclic data without nontermination, and later refresh derived results after fact/rule mutation without rebuilding every table.

The design follows WAM-compatible top-down SLG-style evaluation. It does not directly adopt bottom-up semi-naive Datalog as the core execution model.

## 2. Scope boundaries

### 2.1 Tabling MVP supports

- positive recursive predicates;
- variant call tabling;
- answer dedupe;
- deterministic/finite answer sets;
- in-memory table store first;
- explicit table declarations first.

### 2.2 Tabling MVP excludes

- negation;
- aggregation;
- WFS;
- full answer subsumption;
- tabling all predicates automatically;
- persistent cold table store;
- delta incremental maintenance.

### 2.3 Incremental MVP supports

- mutation event -> stale table invalidation;
- table dependency graph;
- lazy recompute when stale table is queried;
- persistent dependency metadata later.

### 2.4 Incremental MVP excludes

- true differential/delta recomputation;
- concurrent table recompute;
- cross-process table sharing.

## 3. User-facing declaration model

Preferred syntax once parser supports directives:

```prolog
:- table reachable/2.
```

Initial implementation may use API metadata if directive parsing is deferred:

```rust
runtime.declare_tabled(PredicateKey::new("reachable", 2));
```

Rule example:

```prolog
reachable(X, Y) :- edge(X, knows, Y).
reachable(X, Y) :- edge(X, knows, Z), reachable(Z, Y).
```

Query:

```prolog
? reachable(alice, X).
```

Expected with cycle:

```text
terminates, returns finite deduped X values
```

## 4. Core data structures

### 4.1 Table key

A table key identifies a variant call.

```rust
struct TableKey {
    predicate: PredicateKey,
    normalized_args: Vec<NormalizedArg>,
    mode_mask: u64,
    hash: u64,
}

enum NormalizedArg {
    Bound(TermFingerprint),
    Var(usize),
}
```

Variant normalization:

```text
reachable(alice, X) -> reachable/2 [Bound(alice), Var(0)]
reachable(alice, Y) -> same key
reachable(bob, X)   -> different key
reachable(X, Y)     -> reachable/2 [Var(0), Var(1)]
```

### 4.2 Answer tuple

```rust
struct TableAnswer {
    values: Vec<Term>,
    fingerprint: u64,
}
```

For `reachable(alice, X)`, answer values only need to store variables in call order, e.g. `[bob]`, `[carol]`.

### 4.3 Table entry

```rust
enum TableState {
    Evaluating,
    Complete,
    Stale,
    Failed,
}

struct TableEntry {
    key: TableKey,
    state: TableState,
    answers: Vec<TableAnswer>,
    answer_set: HashSet<u64>,
    dependencies: TableDependencies,
    consumers: Vec<ConsumerFrame>,
    generation: u64,
}
```

### 4.4 Dependencies

```rust
struct TableDependencies {
    facts: HashSet<FactKey>,
    rules: HashSet<RuleKey>,
    tables: HashSet<TableKey>,
}
```

### 4.5 Consumer frame

For MVP, avoid fully general suspension first. Use worklist evaluation for tabled predicates. If integrating directly into WAM continuation suspension later:

```rust
struct ConsumerFrame {
    continuation_pc: usize,
    register_snapshot: RegisterFile,
    environment_snapshot: EnvironmentStack,
    heap_checkpoint: usize,
    trail_checkpoint: usize,
    next_answer_index: usize,
}
```

## 5. MVP execution strategy

There are two implementation options.

### Option A: Direct WAM suspension

Modify `Call` for tabled predicates:

1. Compute `TableKey`.
2. If table complete, yield answers through choice points.
3. If table evaluating, suspend current continuation as consumer.
4. If table absent, create producer and evaluate normally.
5. Whenever producer emits new answer, wake consumers.

Pros:

- closest to SLG/XSB;
- integrates with WAM naturally long-term.

Cons:

- high implementation complexity;
- requires continuation suspension/resume correctness.

### Option B: Tabled predicate evaluator wrapper

When querying a tabled predicate, evaluate its strongly connected recursive component using a worklist outside the instruction loop, but still compile rule bodies through WAM for non-recursive goals.

Pros:

- easier MVP;
- avoids deep executor surgery;
- enough for positive graph recursion.

Cons:

- less general;
- still needs careful binding conversion.

### Recommendation

Implement Option B first for deterministic positive recursion, then migrate hot paths into direct WAM suspension if needed.

## 6. Worklist tabling MVP algorithm

### 6.1 Preconditions

- Predicate is explicitly tabled.
- Rule set for predicate is positive.
- Recursive dependencies are known from rule dependency graph.
- No negation or aggregation in SCC.

### 6.2 Compile SCC

For tabled predicate `P`, compute the SCC of predicates mutually recursive with `P`.

```text
build dependency graph from rules
scc = strongly_connected_component(P)
if any negation edge in scc: reject MVP
```

### 6.3 Seed evaluation

For query `P(bound_args...)`:

```text
key = normalize_call(P, args)
if table complete and not stale:
    return table answers
if table stale:
    clear answers and dependencies
mark table evaluating
worklist = [initial_call]
```

### 6.4 Rule expansion loop

```text
while worklist not empty:
    call = pop(worklist)
    for rule in rules_for(call.predicate):
        evaluate_rule_body(rule, call.bindings)
        for each derived answer:
            if answer not in table.answer_set:
                insert answer
                enqueue dependent recursive calls generated by this new answer
mark table complete
```

### 6.5 Evaluating rule bodies

For body goals:

1. Non-tabled storage/index/user predicates: evaluate through existing WAM/provider pipeline.
2. Tabled recursive predicate in same SCC:
   - if call key has answers, join existing answers;
   - if call key has no complete table yet, add call key to worklist;
   - record table dependency.

This resembles top-down memoized fixed-point evaluation while staying inside WAM's rule/provider universe.

## 7. Direct WAM tabling algorithm for later

When moving into the executor:

### 7.1 Call instruction hook

```text
on Call(P/N):
  if P/N is not tabled:
      normal call
  key = normalize registers A1..An
  entry = table_store.get_or_create(key)
  match entry.state:
    Complete:
      install answer choice point over entry.answers
    Evaluating:
      suspend current continuation as consumer
    Stale:
      clear entry and start producer
    Missing/New:
      create producer and evaluate clauses
```

### 7.2 Answer insertion

When a tabled predicate is about to produce a solution:

```text
answer = extract_answer_tuple(current registers, key)
if entry.answer_set.insert(answer.fingerprint):
    entry.answers.push(answer)
    wake_consumers(entry)
else:
    discard duplicate proof path
```

### 7.3 Consumer resume

```text
wake_consumers(entry):
  for consumer in entry.consumers:
      while consumer.next_answer_index < entry.answers.len():
          answer = entry.answers[consumer.next_answer_index]
          consumer.next_answer_index += 1
          restore consumer continuation
          bind answer into registers
          continue execution
```

### 7.4 Completion detection

A table is complete when:

- producer explored all clauses;
- all consumers reached fixpoint;
- no new answers were inserted in the last iteration.

For MVP worklist implementation, completion is simpler: worklist empty and no new answers.

## 8. Dependency tracking for incremental tabling

### 8.1 Recording dependencies

During table evaluation, every provider fact and table answer used by the table must be recorded.

```text
current_table = TableKey(reachable(alice, X))
when edge(alice, knows, bob) used:
    add dependency current_table -> FactKey::Edge(alice, knows, bob)
when recursive table reachable(bob, X) used:
    add dependency current_table -> TableKey(reachable(bob, X))
when rule reachable/2 used:
    add dependency current_table -> RuleKey(reachable/2, hash)
```

### 8.2 Reverse dependency indexes

Maintain reverse maps:

```rust
fact_to_tables: HashMap<FactKey, HashSet<TableKey>>
rule_to_tables: HashMap<RuleKey, HashSet<TableKey>>
table_to_tables: HashMap<TableKey, HashSet<TableKey>>
```

RocksDB persistent keys later:

```text
table:{table_hash} -> TableEntry metadata
table_answer:{table_hash}:{answer_hash} -> encoded answer
dep:fact:{fact_key_hash}:{table_hash} -> ()
dep:rule:{rule_key_hash}:{table_hash} -> ()
dep:table:{dependency_table_hash}:{dependent_table_hash} -> ()
revdep:table:{table_hash}:{dependency_hash} -> ()
```

## 9. Incremental invalidation MVP

### 9.1 Mutation event handling

```text
on MutationEvent(FactDeleted/Inserted/Updated fact_key):
    impacted = fact_to_tables[fact_key]
    mark_stale_recursive(impacted)

on MutationEvent(RuleDeleted/Inserted rule_key):
    impacted = rule_to_tables[rule_key]
    mark_stale_recursive(impacted)
```

### 9.2 Recursive stale marking

```text
mark_stale_recursive(tables):
  stack = tables
  while stack not empty:
      t = pop(stack)
      if table[t].state == Stale: continue
      table[t].state = Stale
      for parent in table_to_tables[t]:
          push(parent)
```

### 9.3 Lazy recompute

```text
on query table key:
  if state == Complete:
      return answers
  if state == Stale:
      clear answers
      clear dependencies
      recompute table from current facts/rules
      return fresh answers
  if missing:
      compute table
```

This is not full delta maintenance, but it prevents stale answers and avoids recomputing unrelated tables.

## 10. Delta incremental maintenance later

After invalidation MVP is stable, implement differential maintenance.

### 10.1 Delta record

```rust
enum DeltaKind { Insert, Delete }

struct FactDelta {
    fact_key: FactKey,
    kind: DeltaKind,
    old_value: Option<Term>,
    new_value: Option<Term>,
}
```

### 10.2 Delta propagation idea

For monotonic positive rules:

```text
new_delta_answers = evaluate affected rules using delta facts joined with stable old facts
insert new answers if not present
repeat until no new delta answers
```

For deletes, either:

- maintain support counts per answer; or
- fallback to stale invalidation recompute.

Recommendation:

```text
insert delta: differential propagation
update/delete delta: stale invalidation until support counts exist
```

## 11. Answer support counts for precise deletes

To avoid full recompute after deletion, track how many proof supports justify each table answer.

```rust
struct SupportedAnswer {
    answer: TableAnswer,
    support_count: u64,
    support_fingerprints: HashSet<u64>,
}
```

When a fact is deleted:

1. find proof supports depending on fact;
2. decrement support counts;
3. remove answer if support reaches zero;
4. propagate negative delta to dependent tables.

This is significantly more complex and should not be MVP.

## 12. Interaction with proof terms

Proof terms and incremental tabling can share dependency metadata:

- proof records explain why an answer exists;
- table dependencies explain when an answer/table becomes stale.

Do not require proof terms before invalidation MVP. Use coarse table-level dependencies first. Later, answer-level dependencies can improve delete precision.

## 13. Interaction with graph builtins

Graph builtins provide a baseline oracle:

```prolog
reachable_builtin(alice, X).
reachable_tabled(alice, X).
```

Tests should compare the tabled recursive rule output with builtin BFS output on the same graph.

## 14. Failure and safety modes

### 14.1 Unsupported rule shape

If a tabled SCC contains unsupported constructs:

- negation;
- cut;
- aggregation;
- impure mutation;
- unbounded arithmetic generation;

MVP should return a structured compile/runtime error:

```text
UnsupportedTabledPredicate("reachable/2 uses negation; stratified tabling not implemented")
```

### 14.2 Limits

Table evaluation must respect:

```rust
struct TableLimits {
    max_tables: usize,
    max_answers_per_table: usize,
    max_iterations: usize,
    timeout_ms: Option<u64>,
}
```

Recommended defaults:

```text
max_tables: 10_000
max_answers_per_table: 1_000_000
max_iterations: 10_000
```

## 15. Verification plan

### 15.1 Base tabling tests

Graph:

```text
a -> b -> c -> a
c -> d
```

Rules:

```prolog
reachable(X, Y) :- edge(X, knows, Y).
reachable(X, Y) :- edge(X, knows, Z), reachable(Z, Y).
```

Expected:

```prolog
? reachable(a, X).
X = b, c, d, a if cycles returning to source are included
```

The query must terminate.

### 15.2 Duplicate answer test

Multiple paths to same node:

```text
a -> b -> d
a -> c -> d
```

Expected:

```prolog
reachable(a, d)
```

appears once.

### 15.3 Incremental invalidation insert

1. Build table for `reachable(a, X)`.
2. Insert edge `d -> e`.
3. Mutation marks table stale.
4. Next query returns `e` additionally.

### 15.4 Incremental invalidation delete

1. Build table for `reachable(a, X)`.
2. Delete edge `c -> d`.
3. Mutation marks table stale.
4. Next query recomputes and removes unreachable answers.

### 15.5 Unrelated mutation

1. Build table for `reachable(a, X)`.
2. Insert edge `x -> y` in disconnected component.
3. Table for `reachable(a, X)` remains complete if no dependency path exists.

## 16. Implementation order

1. Add table declaration metadata.
2. Add `TableKey` normalization and answer tuple canonicalization.
3. Add in-memory `TableStore`.
4. Add dependency graph structures.
5. Implement worklist tabled evaluator for positive recursive SCCs.
6. Add graph reachability equivalence tests against builtin BFS.
7. Add mutation event -> stale invalidation.
8. Add lazy recompute.
9. Persist dependency metadata in RocksDB.
10. Explore delta insert propagation.
