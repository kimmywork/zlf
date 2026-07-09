# Fact Identity, RocksDB Graph Indexing, and Mutation Semantics

## 1. Purpose

This note defines concrete storage/indexing semantics for zlf graph facts. It exists to make these behaviors deterministic and implementation-ready:

- duplicate fact writes are idempotent;
- retraction/deletion is well-defined;
- provider queries can use RocksDB indexes instead of scans;
- future tabling/incremental tabling can depend on stable fact IDs.

## 2. Canonical fact identities

Every durable fact-like write must map to exactly one canonical `FactKey`.

```rust
enum FactKey {
    Node { id: String },
    Label { node: String, label: String },
    Property { entity: String, key: String },
    Edge { source: String, edge_type: String, target: String },
    Rule { predicate: String, arity: usize, source_hash: String },
}
```

### 2.1 Canonicalization rules

| Input syntax | FactKey | Write operation |
|---|---|---|
| `node(Id).` | `Node { Id }` | upsert node if absent |
| `node(Id, Props).` | `Node { Id }` + properties | upsert node + upsert properties |
| `node(Id, Labels, Props).` | node + labels + properties | upsert node, set-add labels, upsert properties |
| `label(Id, Label).` | `Label { Id, Label }` | set-add label |
| `Label(Id).` | `Label { Id, Label }` | set-add label shortcut |
| `property(Id, Key, Value).` | `Property { Id, Key }` | upsert property value |
| `prop_Key(Id, Value).` | `Property { Id, Key }` | upsert property shortcut |
| `edge(S, T, O).` | `Edge { S, T, O }` | upsert edge |
| `edge(S, T, O, Props).` | edge + properties | upsert edge + upsert edge properties |
| `EdgeType(S, O).` | `Edge { S, EdgeType, O }` | upsert edge shortcut |
| `EdgeType(S, O, Props).` | edge + properties | upsert edge shortcut + props |

### 2.2 Set semantics

Storage writers must treat facts as sets, not bags:

```text
apply_fact(F) twice == apply_fact(F) once
```

Query engines may still dedupe final answer bindings because duplicate proof paths can happen even with idempotent storage.

## 3. RocksDB keyspace design

Existing storage can evolve toward these prefix families without changing the Prolog surface.

### 3.1 Primary records

```text
node:{node_id}                         -> Node JSON/bincode
edge:{source}:{type}:{target}          -> Edge JSON/bincode
rule:{predicate}/{arity}:{source_hash} -> CompiledRuleArtifact
```

If explicit edge IDs are later added, keep triple identity as an alternate unique index:

```text
edge_id:{edge_id}                      -> Edge
idx:edge_triple:{source}:{type}:{target} -> edge_id
```

### 3.2 Label indexes

```text
idx:node_label:{label}:{node_id} -> ()
idx:label_node:{node_id}:{label} -> ()
```

Use cases:

| Query | Access path |
|---|---|
| `label(Node, Label)` both vars | scan `idx:label_node:` or node records |
| `label(Node, person)` | scan `idx:node_label:person:` |
| `label(alice, Label)` | scan `idx:label_node:alice:` |
| `person(Node)` | scan `idx:node_label:person:` |

### 3.3 Property indexes

Primary property data may remain embedded in Node/Edge records, but index keys should support bound-key and bound-value lookup.

```text
idx:prop_entity:{entity_id}:{key} -> encoded_value
idx:prop_key:{key}:{entity_id}    -> encoded_value
idx:prop_kv:{key}:{value_hash}:{entity_id} -> encoded_value
```

Use cases:

| Query | Access path |
|---|---|
| `property(alice, Key, Value)` | scan `idx:prop_entity:alice:` |
| `property(Node, name, Value)` | scan `idx:prop_key:name:` |
| `property(Node, name, "Alice")` | scan `idx:prop_kv:name:hash("Alice"):` |
| `prop_name(Node, Name)` | scan `idx:prop_key:name:` |

### 3.4 Edge indexes

At minimum, maintain four indexes:

```text
idx:edge_out:{source}:{type}:{target} -> edge_key
idx:edge_in:{target}:{type}:{source}  -> edge_key
idx:edge_type:{type}:{source}:{target} -> edge_key
idx:edge_any:{source}:{target}:{type} -> edge_key
```

Optional for undirected/schema-backed expansion:

```text
idx:edge_undir:{type}:{min(source,target)}:{max(source,target)} -> edge_key
```

Use cases:

| Query | Access path |
|---|---|
| `edge(alice, Type, Target)` | `idx:edge_out:alice:` |
| `edge(Source, Type, alice)` | `idx:edge_in:alice:` |
| `edge(Source, knows, Target)` | `idx:edge_type:knows:` |
| `knows(alice, Target)` | `idx:edge_out:alice:knows:` |
| `knows(Source, alice)` | `idx:edge_in:alice:knows:` if shortcut supports reverse mode |

## 4. Bound-mode lookup algorithm

Provider resolution should choose the most selective index based on bound arguments.

### 4.1 `edge/3`

```rust
fn resolve_edge(source, edge_type, target) -> Iterator<Edge> {
    match (bound(source), bound(edge_type), bound(target)) {
        (true, true, true) => get_exact_edge(source, edge_type, target),
        (true, true, false) => scan_prefix(idx_edge_out(source, edge_type)),
        (true, false, false) => scan_prefix(idx_edge_out_any_type(source)),
        (false, true, false) => scan_prefix(idx_edge_type(edge_type)),
        (false, true, true) => scan_prefix(idx_edge_in(target, edge_type)),
        (false, false, true) => scan_prefix(idx_edge_in_any_type(target)),
        _ => scan_edges(),
    }
}
```

### 4.2 `property/3`

```rust
fn resolve_property(entity, key, value) -> Iterator<PropertyFact> {
    match (bound(entity), bound(key), bound(value)) {
        (true, true, _) => lookup_entity_property(entity, key),
        (true, false, _) => scan_prefix(idx_prop_entity(entity)),
        (false, true, true) => scan_prefix(idx_prop_kv(key, hash(value))),
        (false, true, false) => scan_prefix(idx_prop_key(key)),
        _ => scan_all_properties(),
    }
}
```

### 4.3 `label/2` and label shortcuts

```rust
fn resolve_label(node, label) -> Iterator<LabelFact> {
    match (bound(node), bound(label)) {
        (true, true) => exact_label(node, label),
        (true, false) => scan_prefix(idx_label_node(node)),
        (false, true) => scan_prefix(idx_node_label(label)),
        (false, false) => scan_all_labels(),
    }
}
```

## 5. Mutation algorithms

### 5.1 Idempotent node upsert

```text
apply node(Id, Labels, Props):
  1. read node:{Id}
  2. if absent: create Node { id, labels: [], properties: {} }
  3. for each label:
       if idx:label_node:{Id}:{label} missing:
           add to node.labels
           put idx:label_node:{Id}:{label}
           put idx:node_label:{label}:{Id}
  4. for each prop:
       old = node.properties[key]
       if old exists: delete idx:prop_kv:{key}:{hash(old)}:{Id}
       set node.properties[key] = value
       put idx:prop_entity:{Id}:{key}
       put idx:prop_key:{key}:{Id}
       put idx:prop_kv:{key}:{hash(value)}:{Id}
  5. put node:{Id}
  6. emit mutation events for inserted/changed keys
```

### 5.2 Idempotent edge upsert

```text
apply edge(S, Type, T, Props):
  1. edge_key = edge:{S}:{Type}:{T}
  2. read edge_key
  3. if absent:
       create edge record
       put idx:edge_out:{S}:{Type}:{T}
       put idx:edge_in:{T}:{Type}:{S}
       put idx:edge_type:{Type}:{S}:{T}
       put idx:edge_any:{S}:{T}:{Type}
  4. merge/upsert properties using property index logic with entity=edge_key or edge_id
  5. emit mutation event Edge{S,Type,T}
```

### 5.3 Retraction/deletion dispatcher

```text
retract(Term):
  parse Term into FactKey or DeletePattern
  dispatch:
    Node -> delete_node
    Label -> remove_label
    Property -> delete_property
    Edge -> delete_edge
  update indexes and emit mutation events
```

### 5.4 Delete node cascade

Default node deletion must cascade incident edges to avoid dangling graph facts.

```text
delete_node(Id):
  1. for edge in scan_prefix(idx:edge_out:{Id}:): delete_edge(edge)
  2. for edge in scan_prefix(idx:edge_in:{Id}:): delete_edge(edge)
  3. for label in scan_prefix(idx:label_node:{Id}:): remove_label(Id,label)
  4. for prop in scan_prefix(idx:prop_entity:{Id}:): delete_property(Id,key)
  5. delete node:{Id}
  6. delete BM25/vector/temporal/index records for Id
  7. emit FactDeleted(Node{Id})
```

## 6. Mutation events for future incremental tabling

Define an internal event stream even if it is initially only in memory.

```rust
enum MutationEvent {
    FactInserted(FactKey),
    FactDeleted(FactKey),
    FactUpdated(FactKey),
    RuleInserted(RuleKey),
    RuleDeleted(RuleKey),
}
```

These events feed later table invalidation:

```text
MutationEvent -> dependency index -> stale table keys -> lazy recompute
```

## 7. Query result dedupe

Even with storage set semantics, rule joins can produce duplicate binding maps. Default graph UX should dedupe final rows.

Algorithm:

```text
dedupe_results(rows):
  seen = HashSet<String>
  out = []
  for row in rows:
    key = canonical_json(row with sorted variable names and canonical terms)
    if key not in seen:
       seen.insert(key)
       out.push(row)
  return out
```

Expose proof/multiplicity later through a separate mode rather than returning duplicate rows by default.

## 8. Verification matrix

| Case | Expected |
|---|---|
| apply `person(alice).` twice | one `person(alice)` result |
| apply `edge(a, knows, b).` twice | one edge record and one `knows(a,b)` result |
| delete label | label shortcut no longer succeeds |
| delete property | `property/3` and `prop_*/2` no longer return it |
| delete edge | `edge/3` and shortcut no longer return it |
| delete node | incident edges disappear |
| update property | old value index is removed; new value index works |
