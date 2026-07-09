# zlf

WAM-backed Prolog graph database for AI-native knowledge bases.

## Overview

zlf stores graph data in RocksDB and exposes that data directly as Prolog facts. Queries are executed by the formal WAM runtime in `zlf-prolog`, not by the legacy AST interpreter.

The main user interfaces are:

- JSON-over-STDIO CLI
- HTTP JSON/SSE streaming server
- Naive Prolog REPL for local inspection

`zlf-api` has been removed; the maintained runtime path is CLI/server over `zlf-query::ZlfDatabase` and `zlf-prolog::wam`.

## Features

- RocksDB-backed property graph storage
- WAM-backed Prolog query execution
- Storage-backed Prolog facts:
  - `node(Id)`
  - `label(Id, Label)`
  - `property(Id, Key, Value)`
  - `edge(Source, Type, Target)`
  - label shortcut: `Person(Node)`
  - edge shortcut: `knows(Source, Target)`
  - property shortcut: `prop_name(Node, Value)`
- Prolog fact write-back to storage:
  - `node(alice).`
  - `node(alice, [person], { name: "Alice" }).`
  - `person(alice).`
  - `knows(alice, bob).`
  - `property(alice, title, "Engineer").`
- Compiled rule persistence via `StorageRuleStore`
- BM25 search with jieba tokenization
- Vector similarity search
- Temporal predicates
- Ollama embedding support, defaulting to `bge-m3:latest`
- Persistent embedding queue and worker loop
- Markdown/wiki import pipeline tests

## Build

```bash
cargo build --release
```

The binary is:

```bash
target/release/zlf
```

## JSON-over-STDIO

Initialize a database:

```bash
echo '{"command":"init","path":"./zlf-db"}' | target/release/zlf
```

Add nodes and edges:

```bash
echo '{"command":"add_node","path":"./zlf-db","labels":["person"],"properties":{"name":"Alice"}}' \
  | target/release/zlf

echo '{"command":"add_edge","path":"./zlf-db","edge_type":"knows","source":"alice","target":"bob","properties":{}}' \
  | target/release/zlf
```

Query with Prolog:

```bash
echo '{"command":"query","path":"./zlf-db","query":"?property(X, name, \"Alice\")."}' \
  | target/release/zlf
```

Index embeddings:

```bash
echo '{"command":"index_embedding","path":"./zlf-db","node_id":"alice","text":"软件工程师"}' \
  | target/release/zlf
```

## Prolog REPL

Start a local REPL:

```bash
target/release/zlf repl ./zlf-db
```

Examples:

```prolog
node(alice, [person, jk], { name: "Alice" }).
node(bob).
person(bob).
knows(alice, bob).
friend(X, Y) :- person(X), person(Y), knows(X, Y).
?person(X).
?friend(alice, bob).
?bm25("软件", Node, Score).
?vector_similar(alice, Node, Score).
```

A successful ground query returns `[{}]` because there are no variables to bind.

## HTTP Server

Start HTTP mode:

```bash
target/release/zlf serve 8520
```

Endpoints:

- `POST /api` — JSON command request/response
- `POST /api/sse` — SSE streaming response for query commands
- `GET /health` — health check

## Configuration

Default config:

```json
{
  "db_path": "./zlf-db",
  "embedding": {
    "provider": "ollama",
    "api_endpoint": "http://localhost:11434",
    "model": "bge-m3:latest",
    "dimension": 1024
  }
}
```

Config load order:

1. `./zlf.json`
2. `~/.zlf/config.json`
3. defaults
4. environment variable overrides

Useful environment variables:

```bash
ZLF_DB_PATH=./zlf-db
ZLF_EMBED_PROVIDER=ollama
ZLF_EMBED_ENDPOINT=http://localhost:11434
OLLAMA_ENDPOINT=http://localhost:11434
ZLF_EMBED_MODEL=bge-m3:latest
ZLF_EMBED_DIMENSION=1024
ZLF_EMBED_API_KEY=...
```

For Ollama:

```bash
ollama pull bge-m3:latest
```

## Prolog Predicates

Storage predicates:

```prolog
node(Id).
label(Id, Label).
property(Id, Key, Value).
edge(Source, Type, Target).
```

Shortcuts:

```prolog
person(alice).              % label shortcut
knows(alice, bob).          % edge shortcut
prop_name(alice, Name).     % property shortcut
```

Index predicates:

```prolog
bm25(Query, Node, Score).
vector_similar(SourceNode, Node, Score).
temporal_on(Date, Node).
temporal_between(StartDate, EndDate, Node).
```

## Development

Run quality checks:

```bash
cargo fmt --all
python3 scripts/check-rust-size.py
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
```

Focused checks used during development:

```bash
cargo test -p zlf-prolog
cargo test -p zlf-cli
cargo test -p zlf-storage
```

Run local Ollama verification:

```bash
cargo test -p zlf-prolog --test ollama_embedding_provider -- --ignored --nocapture
```

Run wiki pipeline verification:

```bash
cargo test -p zlf-prolog --test wiki_full_pipeline -- --ignored --nocapture
```

## License

MIT OR Apache-2.0
