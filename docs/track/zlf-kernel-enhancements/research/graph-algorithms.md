# Storage-backed Graph Algorithm Builtins

## 1. Purpose

This note defines how zlf should implement graph algorithm predicates on top of RocksDB-backed graph indexes. The goal is to provide cycle-safe graph functionality before full tabling is available, while also creating validation workloads for later tabling.

## 2. Required RocksDB indexes

Graph algorithms must not scan all edges on every step. They require adjacency indexes.

```text
idx:edge_out:{source}:{type}:{target} -> edge_key
idx:edge_in:{target}:{type}:{source}  -> edge_key
idx:edge_type:{type}:{source}:{target} -> edge_key
```

For untyped traversal, scan all type prefixes under a source:

```text
idx:edge_out:{source}:
```

For typed traversal:

```text
idx:edge_out:{source}:{type}:
```

## 3. Builtin predicate semantics

### 3.1 `neighbors/2`

```prolog
neighbors(Node, Neighbor).
```

Default: outgoing directed neighbors.

Equivalent to:

```prolog
edge(Node, _, Neighbor).
```

But implemented with direct `idx:edge_out:{Node}:` prefix scan.

Modes:

| Mode | Implementation |
|---|---|
| `neighbors(+Node, -Neighbor)` | prefix scan source adjacency |
| `neighbors(+Node, +Neighbor)` | exact edge existence scan over source and target filter |
| `neighbors(-Node, -Neighbor)` | scan `idx:edge_out:` |

### 3.2 `neighbors/3`

```prolog
neighbors(Node, Type, Neighbor).
```

Equivalent to:

```prolog
edge(Node, Type, Neighbor).
```

Access:

```text
idx:edge_out:{Node}:{Type}:
```

### 3.3 Degree predicates

```prolog
degree(Node, Degree).
in_degree(Node, Degree).
out_degree(Node, Degree).
```

Definitions:

```text
out_degree(N) = count idx:edge_out:{N}:
in_degree(N)  = count idx:edge_in:{N}:
degree(N)     = in_degree(N) + out_degree(N)
```

For exact graph-theoretic undirected degree, a later predicate should deduplicate reciprocal edges. The default is directed degree count.

### 3.4 Reachability

```prolog
reachable(Source, Target).
reachable(Source, Target, MaxDepth).
```

`reachable/2` should use a safe default max depth or query limit until tabling is stable. Recommended:

```text
reachable/2 = reachable/3 with max_depth from config, default 32
```

This avoids unbounded work in large/cyclic graphs.

### 3.5 Shortest path

```prolog
shortest_path(Source, Target, Path).
```

Returns one shortest path as a list of node IDs:

```json
{ "Path": ["alice", "bob", "carol"] }
```

If multiple shortest paths exist, MVP returns the first stable lexicographic/index-order path. Later extension can expose all shortest paths.

## 4. Algorithm details

### 4.1 Neighbor iteration

```rust
fn outgoing_neighbors(source: &str, edge_type: Option<&str>) -> Iterator<NodeId> {
    match edge_type {
        Some(t) => scan_prefix(format!("idx:edge_out:{source}:{t}:")),
        None => scan_prefix(format!("idx:edge_out:{source}:")),
    }
    .map(parse_target_from_key)
}
```

To avoid returning duplicate neighbors when multiple edge types connect the same target, `neighbors/2` may dedupe by target. `edge/3` should not dedupe different edge types.

### 4.2 Bounded BFS for `reachable/3`

```text
reachable(Source, Target, MaxDepth):
  if Source == Target:
      return true with path length 0 if zero-length reachability is enabled
  visited = { Source }
  queue = [(Source, 0)]
  while queue not empty:
      (node, depth) = pop_front(queue)
      if depth == MaxDepth: continue
      for next in outgoing_neighbors(node):
          if next == Target: return true
          if next not in visited:
              visited.insert(next)
              push_back(queue, (next, depth + 1))
  return false
```

### 4.3 Enumerating `reachable(Source, Target, MaxDepth)` when Target is variable

```text
reachable(+Source, -Target, +MaxDepth):
  visited = { Source }
  queue = [(Source, 0)]
  results = []
  while queue not empty:
      (node, depth) = pop_front(queue)
      if depth == MaxDepth: continue
      for next in outgoing_neighbors(node):
          if next not in visited:
              visited.insert(next)
              results.push(next)
              push_back(queue, (next, depth + 1))
  return results as individual Prolog facts reachable(Source, Target, MaxDepth)
```

Do not return `Source` itself unless a cycle reaches it or a separate option enables zero-length reachability.

### 4.4 Handling variable Source

For MVP, support these modes first:

```text
reachable(+Source, -Target)
reachable(+Source, +Target)
reachable(+Source, -Target, +MaxDepth)
reachable(+Source, +Target, +MaxDepth)
```

Defer fully variable source enumeration for large graphs because it can become all-pairs reachability.

If needed later:

```text
reachable(-Source, -Target, +MaxDepth):
  for each source in node index:
      run bounded BFS
```

This must respect result limits.

### 4.5 Shortest path BFS

```text
shortest_path(Source, Target):
  if Source == Target: return [Source]
  visited = { Source }
  parent = Map<Node, Node>
  queue = [Source]
  while queue not empty:
      node = pop_front(queue)
      for next in outgoing_neighbors(node):
          if next in visited: continue
          visited.insert(next)
          parent[next] = node
          if next == Target:
              return reconstruct_path(parent, Source, Target)
          push_back(queue, next)
  fail
```

Path reconstruction:

```text
path = [Target]
while current != Source:
  current = parent[current]
  path.push(current)
reverse(path)
```

### 4.6 Degree counting

Use prefix count. MVP can count by iterating prefix keys. Later optimize by maintaining counters:

```text
cnt:edge_out:{node} -> u64
cnt:edge_in:{node}  -> u64
cnt:edge_type:{type}:{node}:out -> u64
cnt:edge_type:{type}:{node}:in  -> u64
```

Counters must update transactionally with edge insert/delete.

## 5. Provider integration

Graph algorithms should be exposed through `FactProvider` as generated facts.

Example for `reachable(alice, Target, 3)`:

```rust
fn facts_for_reachable(args) -> Vec<Term> {
    let source = bound_atom(args[0])?;
    let max_depth = bound_usize(args[2])?;
    bfs_reachable(source, max_depth)
      .map(|target| compound("reachable", [atom(source), atom(target), number(max_depth)]))
      .collect()
}
```

For `shortest_path/3`, return one fact:

```prolog
shortest_path(alice, carol, [alice, bob, carol]).
```

## 6. Result limits and safety

All graph builtins must honor global query limits:

```rust
struct GraphQueryOptions {
    max_depth: usize,
    max_visited: usize,
    max_results: usize,
    timeout_ms: Option<u64>,
}
```

Recommended defaults:

```text
max_depth for reachable/2: 32
max_visited: 100_000
max_results: 10_000
```

If a limit is hit, return a structured error instead of silently truncating unless the query explicitly asks for limit semantics.

## 7. Interaction with tabling

Graph builtins are not a replacement for tabling. They serve three roles:

1. immediate graph-query value;
2. safe path queries before recursive Prolog tabling exists;
3. reference oracle tests for later tabled recursive rules.

Example later equivalence test:

```prolog
path(X, Y) :- edge(X, knows, Y).
path(X, Y) :- edge(X, knows, Z), path(Z, Y).
```

For a tabled `path/2`, compare answers against `reachable/2` on the same graph.

## 8. Verification fixtures

### 8.1 Linear graph

```text
a -> b -> c -> d
```

Expected:

```prolog
reachable(a, d, 3) succeeds
shortest_path(a, d, [a,b,c,d])
out_degree(b, 1)
in_degree(b, 1)
```

### 8.2 Cycle graph

```text
a -> b -> c -> a
```

Expected:

```prolog
reachable(a, c, 3) succeeds
reachable(a, d, 3) fails
query terminates
```

### 8.3 Branching graph

```text
a -> b -> d
a -> c -> d
```

Expected:

```prolog
shortest_path(a, d, Path) returns length 3
reachable(a, X, 2) returns b,c,d with dedupe
```

## 9. Implementation order

1. Ensure edge out/in indexes exist and are maintained.
2. Implement `neighbors/2` and `neighbors/3`.
3. Implement degree predicates using prefix counts.
4. Implement `reachable/3` bounded BFS.
5. Define safe default for `reachable/2`.
6. Implement `shortest_path/3`.
7. Add result limit handling.
8. Add tests comparing graph builtins and ordinary `edge/3` facts.
