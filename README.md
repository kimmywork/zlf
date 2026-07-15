# zlf

**Zenith Logic Foundry**

> *Forge answers from everything.*
>
> 解答世间万物

zlf is a WAM-backed Prolog graph database for building AI-native knowledge bases. It stores graph facts persistently, evaluates queries and rules through a single Prolog runtime, and supports retrieval over the resulting knowledge graph.

## What it provides

- Persistent property-graph storage backed by RocksDB
- Prolog facts, rules, and queries executed by a WAM runtime
- BM25 retrieval, optional vector similarity, and temporal queries
- JSON-over-STDIO CLI, HTTP/SSE server, and interactive Prolog REPL
- Compiled-rule persistence and storage-backed fact writes

## Quick start

Build the CLI:

```bash
cargo build --release
```

Open a database in the Prolog REPL:

```bash
target/release/zlf repl ./zlf-db
```

## Usage

See the [usage guide](docs/usage-guide.md) for installation, configuration, REPL and Prolog examples, JSON-over-STDIO commands, HTTP endpoints, index profiles, embeddings, and operational guidance.

## Architecture

```text
CLI / HTTP server / REPL
  -> WAM Prolog runtime
  -> storage-backed facts, compiled rules, and indexes
  -> RocksDB
```

## Development

```bash
cargo fmt --all
python3 scripts/check-rust-size.py
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
```

## License

MIT OR Apache-2.0
