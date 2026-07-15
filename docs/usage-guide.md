# zlf Usage Guide

zlf is a RocksDB-backed graph database with a single WAM Prolog runtime. Its primary interfaces are:

- JSON-over-STDIO: `zlf`
- HTTP: `zlf serve [port]`
- interactive Prolog: `zlf repl [db_path]`


## Table of contents

1. [Build and configuration](#build-and-configuration)
2. [Quick start with the Prolog REPL](#quick-start-with-the-prolog-repl)
3. [Using Prolog in the REPL](#using-prolog-in-the-repl)
4. [IndexProfile](#indexprofile)
5. [JSON-over-STDIO](#json-over-stdio)
6. [HTTP server](#http-server)
7. [Errors and operational guidance](#errors-and-operational-guidance)

## Build and configuration

```bash
git clone <repository-url>
cd zlf
cargo build --release
```

The executable is `target/release/zlf`.

The configuration is loaded from `./zlf.json`, then `~/.zlf/config.json`, with environment overrides applied last. The default is:

```json
{
  "db_path": "./zlf-db",
  "embedding": {
    "enabled": false,
    "index_engine": "exact",
    "provider": "ollama",
    "api_endpoint": "http://localhost:11434",
    "api_key": null,
    "model": "bge-m3:latest",
    "dimension": 1024
  }
}
```

Embedding is deliberately disabled by default. Enable it only for workloads that benefit from semantic retrieval:

```json
{
  "db_path": "./zlf-db",
  "embedding": {
    "enabled": true,
    "index_engine": "hnsw",
    "provider": "ollama",
    "api_endpoint": "http://localhost:11434",
    "model": "bge-m3:latest",
    "dimension": 1024
  }
}
```

`index_engine` is `exact` or `hnsw`. HNSW always retains exact RocksDB as source of truth and fallback.

Environment overrides:

```bash
ZLF_DB_PATH=./zlf-db
ZLF_EMBED_ENABLED=true
ZLF_VECTOR_INDEX_ENGINE=hnsw
ZLF_EMBED_PROVIDER=ollama
ZLF_EMBED_ENDPOINT=http://localhost:11434
OLLAMA_ENDPOINT=http://localhost:11434
ZLF_EMBED_MODEL=bge-m3:latest
ZLF_EMBED_DIMENSION=1024
ZLF_EMBED_API_KEY=...
```

## Quick start with the Prolog REPL

Start or reopen a database:

```bash
target/release/zlf repl ./example-db
```

The REPL accepts facts, rules, directives, and queries:

```prolog
node(alice, [person], { name: "Alice", title: "Engineer" }).
node(bob, [person], { name: "Bob", title: "Engineer" }).
knows(alice, bob).

friend(X, Y) :- person(X), person(Y), knows(X, Y).

? person(X).
? property(alice, name, Name).
? friend(alice, Who).
```

Exit with `:quit`, `:exit`, or Ctrl-D. Use `:help` for examples. A successful ground query has no variable bindings and is printed as `[{}]`.

## Using Prolog in the REPL

### Input forms

Facts are persisted through the canonical storage writer:

```prolog
node(alice).
node(alice, [person, employee], { name: "Alice", age: 30 }).
person(alice).
knows(alice, bob).
property(alice, title, "Staff Engineer").
```

Rules are compiled and persisted through `StorageRuleStore`:

```prolog
colleague(X, Y) :- works_at(X, C), works_at(Y, C), X \= Y.
reachable(X, Y) :- knows(X, Y).
reachable(X, Y) :- knows(X, Z), reachable(Z, Y).
```

Queries start with `?`:

```prolog
? node(Id).
? person(Id).
? property(Id, name, Name).
? edge(Source, Type, Target).
? knows(alice, Who).
? prop_name(Id, Name).
```

A line can contain multiple facts:

```prolog
node(a). node(b). follows(a, b).
```

### Current storage predicates

```prolog
node(Id).
label(Id, Label).
property(Id, Key, Value).
edge(Source, Type, Target).
```

Shortcuts are derived from labels, edge types, and properties:

```prolog
person(alice).             % label shortcut
knows(alice, bob).         % edge-type shortcut
prop_name(alice, Name).    % property shortcut
```

### Mutation and retraction

```prolog
? retract(person(alice)).
? retract(edge(alice, knows, bob)).
? retract(prop_title(alice, _)).
```

Writing `Label(Id)` to an existing node merges the label and preserves existing properties.

### Index predicates

```prolog
? bm25("software engineer", Node, Score).
? vector_similar(alice, Node, Score).
? temporal_on("2026-07-15", Node).
? temporal_between("2026-07-01", "2026-08-01", Node).
? valid_at("2026-07-15T00:00:00Z", Node).
? valid_overlaps("2026-07-01T00:00:00Z", "2026-08-01T00:00:00Z", Node).
```

`vector_similar/3` returns an explicit index-unavailable error when embedding is disabled. BM25, vector, and temporal indexes only contain fields selected by an active IndexProfile.

### IndexProfile directives in Prolog

A BM25 profile can be created and activated directly in the REPL:

```prolog
:- index_profile(knowledge, 1, {
  matcher: { node_labels: { labels: [document] } },
  fields: {
    body: {
      bm25: {
        analyzer_id: "unicode_jieba_v1",
        analyzer_version: 1,
        weight: 1.0,
        k1: 1.2,
        b: 0.75
      }
    }
  }
}).

:- activate_index_profile(knowledge, 1).
```

Profiles are immutable by `(name, version)`. Change the version instead of modifying an existing artifact.

## IndexProfile

An `IndexProfileArtifact` selects entities and fields and declares which derivative indexes each field feeds.

### Artifact fields

| Field | Meaning |
|---|---|
| `schema_version` | Currently `1` |
| `name` / `version` | Immutable profile identity |
| `source_hash` | Canonical profile hash; the CLI accepts `""` and computes it |
| `matcher` | `node_labels` or `edge_types` |
| `fields` | Map from property name to BM25/vector/temporal options |
| `created_at` | RFC 3339 timestamp |

Matcher forms:

```json
{"node_labels": {"labels": ["document"]}}
```

```json
{"edge_types": {"edge_types": ["mentions"]}}
```

### BM25 options

The current Tantivy contract pins `k1=1.2`, `b=0.75`, analyzer ID `unicode_jieba_v1`, and analyzer version `1`. `weight` must be positive.

```json
{
  "bm25": {
    "analyzer_id": "unicode_jieba_v1",
    "language": "en",
    "analyzer_version": 1,
    "weight": 1.0,
    "k1": 1.2,
    "b": 0.75
  }
}
```

### Vector options

Vector fields require `embedding.enabled=true`:

```json
{
  "vector": {
    "model_profile": "bge_m3_dense_v1",
    "chunking": {"whole_field": {"version": 1}}
  }
}
```

Supported chunking forms:

```json
{"explicit": {"version": 1}}
{"whole_field": {"version": 1}}
{"paragraph_heading": {"version": 1}}
{"fixed_token_window": {"version": 1, "size": 128, "overlap": 16}}
```

### Temporal options

A field may be an event instant, validity start, or validity end:

```json
{"temporal": "event"}
{"temporal": "valid_from"}
{"temporal": "valid_to"}
```

At most one `valid_from` and one `valid_to` field are allowed; `valid_to` requires `valid_from`.

### Complete JSON profile

```json
{
  "schema_version": 1,
  "name": "knowledge",
  "version": 1,
  "source_hash": "",
  "matcher": {"node_labels": {"labels": ["document"]}},
  "fields": {
    "title": {
      "bm25": {
        "analyzer_id": "unicode_jieba_v1",
        "analyzer_version": 1,
        "weight": 2.0,
        "k1": 1.2,
        "b": 0.75
      }
    },
    "body": {
      "bm25": {
        "analyzer_id": "unicode_jieba_v1",
        "analyzer_version": 1,
        "weight": 1.0,
        "k1": 1.2,
        "b": 0.75
      }
    },
    "published_at": {"temporal": "event"}
  },
  "created_at": "2026-07-15T00:00:00Z"
}
```

Install, activate, list, and inspect lifecycle status through JSON-over-STDIO:

```json
{"command":"put_index_profile","path":"./zlf-db","profile":{...}}
{"command":"activate_index_profile","path":"./zlf-db","name":"knowledge","version":1}
{"command":"list_index_profiles","path":"./zlf-db"}
{"command":"index_status","path":"./zlf-db","target":"bm25"}
{"command":"wait_indexes","path":"./zlf-db","targets":["bm25"],"minimum_sequence":1,"timeout_ms":1000}
```

Activation rebuilds and publishes the relevant generation. A profile containing vector fields is rejected while embedding is disabled.

## JSON-over-STDIO

Send one JSON request per line to `target/release/zlf`. Responses are also one JSON object per line.

```bash
echo '{"command":"init","path":"./zlf-db"}' | target/release/zlf
```

Current commands:

| Command | Purpose |
|---|---|
| `init` | Initialize a database |
| `add_node`, `get_node` | Create/read nodes |
| `add_edge`, `get_edge`, `edge_ids` | Create/read/resolve edges |
| `patch_node_properties`, `set_node_property`, `remove_node_property` | Mutate node properties |
| `patch_edge_properties`, `set_edge_property`, `remove_edge_property` | Mutate edge properties |
| `query` | Execute a Prolog query/directive/rule |
| `search` | BM25 text search |
| `put_index_profile`, `activate_index_profile`, `list_index_profiles` | Manage profiles |
| `index_status`, `wait_indexes` | Observe index lifecycle |
| `vector_index_status`, `rebuild_vector_index` | Observe/request optional HNSW publication |
| `import`, `export` | JSON graph import/export |
| `embed` | Generate an embedding when enabled |
| `config` | Read/write `zlf.json` configuration |

Examples:

```bash
echo '{"command":"add_node","path":"./zlf-db","labels":["person"],"properties":{"name":"Alice"}}' | target/release/zlf

echo '{"command":"query","path":"./zlf-db","query":"? property(Id, name, Name)."}' | target/release/zlf

echo '{"command":"search","path":"./zlf-db","query":"software engineer"}' | target/release/zlf
```

Success and error envelopes:

```json
{"type":"success","data":{}}
```

```json
{"type":"error","code":"QUERY_FAILED","message":"..."}
```

## HTTP server

```bash
target/release/zlf serve 8520
```

- `POST /api` — the same JSON command protocol
- `POST /api/sse` — SSE query responses
- `GET /health` — health check

## Errors and operational guidance

- `INDEX_UNAVAILABLE` or an `Index 'vector_embedding' ...` message means embedding/vector indexing is disabled for the requested operation.
- HNSW is optional. Missing, stale, rebuilding, incompatible, or corrupt ANN state automatically falls back to exact vectors.
- For large semantic knowledge imports, finish a batch and its embeddings before requesting one HNSW rebuild:

```bash
echo '{"command":"rebuild_vector_index","path":"./zlf-db"}' | target/release/zlf
echo '{"command":"vector_index_status","path":"./zlf-db"}' | target/release/zlf
```

- For symbol-heavy code repositories, prefer BM25 and graph relationships and leave embedding disabled unless measurements show a clear benefit.
