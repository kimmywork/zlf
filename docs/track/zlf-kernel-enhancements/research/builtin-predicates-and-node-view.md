# Builtin Predicates, Predicate Registry, and Node View Semantics

## 1. Purpose

This note defines the exact Prolog-facing predicate contracts for zlf. It answers:

- which predicates are builtin vs provider-backed vs user-defined;
- how node labels/properties/edges are queried;
- how node view predicates should shape their output;
- how rule/predicate introspection should work.

## 2. Predicate categories

```rust
enum PredicateKind {
    BuiltinCore,
    StorageProvider,
    IndexProvider,
    LabelShortcut,
    EdgeShortcut,
    PropertyShortcut,
    UserRule,
    GraphAlgorithm,
    Introspection,
}
```

## 3. Current and planned predicate catalog

### 3.1 Storage provider predicates

| Predicate | Kind | Meaning |
|---|---|---|
| `node/1` | StorageProvider | `node(Id)` succeeds when node exists. |
| `label/2` | StorageProvider | `label(Id, Label)` enumerates or checks node labels. |
| `property/3` | StorageProvider | `property(Entity, Key, Value)` enumerates or checks node/edge properties. |
| `edge/3` | StorageProvider | `edge(Source, Type, Target)` enumerates/checks directed edge triples. |

### 3.2 Dynamic shortcuts

| Predicate | Kind | Expansion |
|---|---|---|
| `person(Id)` | LabelShortcut | `label(Id, person)` |
| `knows(Source, Target)` | EdgeShortcut | `edge(Source, knows, Target)` |
| `prop_name(Entity, Value)` | PropertyShortcut | `property(Entity, name, Value)` |

Shortcut discovery is dynamic:

- label shortcuts come from labels in storage;
- edge shortcuts come from edge types in storage;
- property shortcuts come from property keys in storage.

### 3.3 Index provider predicates

| Predicate | Kind | Meaning |
|---|---|---|
| `bm25/3` | IndexProvider | `bm25(Query, Node, Score)` text search result. |
| `vector_similar/3` | IndexProvider | `vector_similar(Source, Node, Score)` vector similarity. |
| `temporal_on/2` | IndexProvider | `temporal_on(Date, Node)` exact date match. |
| `temporal_between/3` | IndexProvider | `temporal_between(Start, End, Node)` range match. |

### 3.4 Planned graph view predicates

| Predicate | Kind | Meaning |
|---|---|---|
| `labels/2` | StorageProvider | `labels(Node, Labels)` returns all labels as a list. |
| `properties/2` | StorageProvider | `properties(Entity, Props)` returns all properties as an object. |
| `out_edges/2` | StorageProvider | `out_edges(Node, Edges)` returns outgoing edge objects. |
| `out_edges/3` | StorageProvider | `out_edges(Node, Type, Edges)` returns outgoing edges of type. |
| `in_edges/2` | StorageProvider | `in_edges(Node, Edges)` returns incoming edge objects. |
| `in_edges/3` | StorageProvider | `in_edges(Node, Type, Edges)` returns incoming edges of type. |
| `neighbors/2` | StorageProvider | `neighbors(Node, Neighbor)` enumerates adjacent nodes. |
| `neighbors/3` | StorageProvider | `neighbors(Node, Type, Neighbor)` enumerates adjacent nodes by edge type. |
| `node_view/2` | StorageProvider | `node_view(Node, View)` returns labels/properties/in/out edge summary. |

### 3.5 Planned introspection predicates

| Predicate | Kind | Meaning |
|---|---|---|
| `predicate/3` | Introspection | `predicate(Name, Arity, Kind)` lists known predicates. |
| `builtin_predicate/3` | Introspection | `builtin_predicate(Name, Arity, Description)`. |
| `rule/3` | Introspection | `rule(Name, Arity, Source)` lists user rules. |
| `rule_depends_on/2` | Introspection | `rule_depends_on(Rule, Dependency)`. |

### 3.6 Planned graph algorithm predicates

| Predicate | Kind | Meaning |
|---|---|---|
| `reachable/2` | GraphAlgorithm | `reachable(Source, Target)` unbounded but cycle-safe reachability. |
| `reachable/3` | GraphAlgorithm | `reachable(Source, Target, MaxDepth)` bounded reachability. |
| `shortest_path/3` | GraphAlgorithm | `shortest_path(Source, Target, Path)`. |
| `degree/2` | GraphAlgorithm | total degree. |
| `in_degree/2` | GraphAlgorithm | incoming degree. |
| `out_degree/2` | GraphAlgorithm | outgoing degree. |

## 4. Exact semantics for node view predicates

### 4.1 `labels/2`

```prolog
labels(Node, Labels).
```

Modes:

| Mode | Behavior |
|---|---|
| `labels(+Node, -Labels)` | Returns a single row containing all labels as a list. |
| `labels(-Node, -Labels)` | Enumerates nodes with their label lists. |
| `labels(+Node, +Labels)` | Checks exact set equality after canonical sorting. |

Output example:

```json
{ "Labels": ["person", "developer"] }
```

Implementation:

1. If `Node` bound: scan `idx:label_node:{Node}:`.
2. If `Node` unbound: iterate node records or `idx:label_node:` grouped by node.
3. Sort labels lexicographically for stable output.
4. Return `Term::List([Atom(label), ...])`.

### 4.2 `properties/2`

```prolog
properties(Entity, Props).
```

Modes:

| Mode | Behavior |
|---|---|
| `properties(+Entity, -Props)` | Return all properties for node or edge entity. |
| `properties(-Entity, -Props)` | Enumerate all entities with properties. |
| `properties(+Entity, +Props)` | Exact object equality check. |

Output example:

```json
{ "Props": { "name": "Alice", "age": 30 } }
```

Implementation:

1. If `Entity` bound: scan `idx:prop_entity:{Entity}:`.
2. Decode values into `Term::Object(Vec<(String, Term)>)`.
3. Sort object keys for stable equality/dedup.

### 4.3 Edge object shape

Edge list predicates return edge objects using `Term::Object`.

```json
{
  "id": "alice:knows:bob",
  "source": "alice",
  "type": "knows",
  "target": "bob",
  "properties": { "since": 2020 }
}
```

Term form:

```prolog
{
  id: "alice:knows:bob",
  source: alice,
  type: knows,
  target: bob,
  properties: { since: 2020 }
}
```

### 4.4 `out_edges/2` and `out_edges/3`

```prolog
out_edges(Node, Edges).
out_edges(Node, Type, Edges).
```

Modes:

| Mode | Access path |
|---|---|
| `out_edges(+Node, -Edges)` | scan `idx:edge_out:{Node}:` |
| `out_edges(+Node, +Type, -Edges)` | scan `idx:edge_out:{Node}:{Type}:` |
| `out_edges(-Node, -Edges)` | group `idx:edge_out:` by source |

Return one row per node/type group, not one row per edge. For per-edge enumeration, use `edge/3`.

### 4.5 `in_edges/2` and `in_edges/3`

```prolog
in_edges(Node, Edges).
in_edges(Node, Type, Edges).
```

Modes mirror `out_edges`, but use `idx:edge_in:`.

### 4.6 `neighbors/2` and `neighbors/3`

```prolog
neighbors(Node, Neighbor).
neighbors(Node, Type, Neighbor).
```

Default semantics: outgoing neighbors only.

Rationale: zlf edges are directed unless schema says otherwise. For incoming neighbors, use `edge(Neighbor, Type, Node)` or future `in_neighbors/2`.

Modes:

| Mode | Behavior |
|---|---|
| `neighbors(+Node, -Neighbor)` | Enumerates outgoing targets. |
| `neighbors(+Node, +Type, -Neighbor)` | Enumerates outgoing targets by edge type. |
| `neighbors(-Node, -Neighbor)` | Enumerates all directed source-target pairs. |

Future optional predicates:

```prolog
in_neighbors(Node, Neighbor).
undirected_neighbors(Node, Neighbor).
```

### 4.7 `node_view/2`

```prolog
node_view(Node, View).
```

Output shape:

```json
{
  "id": "alice",
  "labels": ["person"],
  "properties": { "name": "Alice" },
  "out_edges": [ ...edge objects... ],
  "in_edges": [ ...edge objects... ]
}
```

Term shape:

```prolog
{
  id: alice,
  labels: [person],
  properties: { name: "Alice" },
  out_edges: [...],
  in_edges: [...]
}
```

Modes:

| Mode | Behavior |
|---|---|
| `node_view(+Node, -View)` | Return one view if node exists. |
| `node_view(-Node, -View)` | Enumerate views for all nodes. |
| `node_view(+Node, +View)` | Exact view check. |

Implementation should avoid N+1 scans by collecting labels/properties/out/in edges from indexed prefixes.

## 5. Predicate registry implementation

### 5.1 Registry data model

```rust
struct PredicateDescriptor {
    name: String,
    arity: usize,
    kind: PredicateKind,
    description: String,
    source: PredicateSource,
}

enum PredicateSource {
    Builtin,
    StorageProvider,
    IndexProvider,
    Shortcut { base: PredicateKey },
    UserRule { rule_key: String },
}
```

### 5.2 Registry construction

At query time, build a composite registry from:

1. static builtin descriptors;
2. storage provider discovered labels/edge types/property keys;
3. rule store entries;
4. graph algorithm builtin descriptors.

For performance, cache the registry per database revision later. Initially, rebuild on demand.

### 5.3 `predicate/3`

```prolog
predicate(Name, Arity, Kind).
```

Examples:

```prolog
predicate(node, 1, storage).
predicate(edge, 3, storage).
predicate(person, 1, label_shortcut).
predicate(knows, 2, edge_shortcut).
predicate(prop_name, 2, property_shortcut).
predicate(friend, 2, user_rule).
```

### 5.4 `builtin_predicate/3`

Only static builtins and graph algorithm builtins should appear here, not user rules.

```prolog
builtin_predicate(node, 1, "True when node exists").
builtin_predicate(edge, 3, "Directed graph edge").
builtin_predicate(reachable, 3, "Bounded graph reachability").
```

### 5.5 `rule/3`

```prolog
rule(Name, Arity, Source).
```

Examples:

```prolog
rule(friend, 2, "friend(X, Y) :- knows(X, Y).").
```

Rule source should come from `CompiledRuleArtifact.source`.

### 5.6 `rule_depends_on/2`

Represent predicate indicators as atoms for now:

```prolog
rule_depends_on("friend/2", "knows/2").
```

Later, if parser supports `/` as a structured operator, expose `friend/2` as a compound/predicate indicator term.

Dependency extraction:

```text
for each rule:
  head_key = predicate_key(rule.head)
  for each body goal:
     dep_key = predicate_key(goal)
     emit rule_depends_on(head_key, dep_key)
```

Ignore control/builtin goals such as `!` for dependency purposes unless they are relevant to optimization metadata.

## 6. Safe undirected edge modeling

Do not recommend direct recursion:

```prolog
friend(X, Y) :- friend(Y, X).  % unsafe without base/table
```

Recommended pattern:

```prolog
friend(X, Y) :- friend_edge(X, Y).
friend(X, Y) :- friend_edge(Y, X).
```

Storage remains directed. The symmetric predicate is derived.

Future schema option:

```prolog
edge_type(friend_edge, { directed: false }).
```

If schema says `directed: false`, provider can expose both orientations for shortcut predicates.

## 7. Verification matrix

| Query | Expected |
|---|---|
| `predicate(Name, Arity, Kind)` | lists builtins, shortcuts, rules |
| `builtin_predicate(edge, 3, Desc)` | succeeds |
| `rule(friend, 2, Source)` | returns persisted source |
| `rule_depends_on("friend/2", "knows/2")` | succeeds |
| `labels(alice, Labels)` | one list of labels |
| `properties(alice, Props)` | one object of properties |
| `out_edges(alice, Edges)` | one list of outgoing edge objects |
| `neighbors(alice, X)` | enumerates outgoing adjacent nodes |
| `node_view(alice, View)` | returns full node summary |
