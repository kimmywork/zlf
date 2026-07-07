# zlf

AI-Native Graph Database with Logic Reasoning

## Overview

zlf is a lightweight graph database that combines property graph storage with Prolog-style logic reasoning, designed specifically for AI agents and knowledge management.

## Features

- **Property Graph Storage**: Nodes with labels and properties, edges with types
- **Prolog-style Query Language**: Declarative rules for complex relationship queries
- **BM25 Full-text Search**: Chinese and English text search with jieba tokenization
- **Semantic Search**: Vector similarity search with pluggable embedding providers
- **Temporal Query Support**: Time-range filters and node versioning
- **JSON-over-STDIO Interface**: Simple CLI and TypeScript SDK

## Quick Start

### Build from Source

```bash
# Build the Rust CLI binary
cargo build --release

# The binary will be at target/release/zlf
```

### Initialize a Database

```bash
echo '{"command":"init","path":"./my-db"}' | ./target/release/zlf
```

### Add Nodes

```bash
echo '{"command":"add_node","path":"./my-db","labels":["person"],"properties":{"name":"Alice","age":30}}' | ./target/release/zlf
```

### Query Data

```bash
echo '{"command":"query","path":"./my-db","query":"node(person, X, Props)."}' | ./target/release/zlf
```

## TypeScript SDK

```typescript
import { ZLF } from 'zlf';

const db = new ZLF('./my-db');

// Add a node
const node = await db.addNode(['person'], { name: 'Alice', age: 30 });

// Get a node
const retrieved = await db.getNode(node.id);

// Add an edge
const bob = await db.addNode(['person'], { name: 'Bob' });
const edge = await db.addEdge('knows', node.id, bob.id, { since: 2020 });

// Query
const results = await db.query('node(person, X, Props).');

// Search
const searchResults = await db.search('software engineer');

// Memory operations
await db.storeMemory('conv123', {
  type: 'conversation',
  content: { message: 'Hello' },
  entities: ['alice', 'bob'],
  importance: 0.8
});
```

## CLI Commands (JSON-over-STDIO)

| Command | Description |
|---------|-------------|
| `init` | Initialize a new database |
| `add_node` | Add a node with labels and properties |
| `get_node` | Get a node by ID |
| `add_edge` | Add an edge between two nodes |
| `get_edge` | Get an edge by ID |
| `query` | Execute a zlf-log query |
| `search` | BM25 full-text search |
| `similar` | Semantic similarity search |
| `import` | Import data from JSON file |
| `export` | Export data to JSON format |

## Architecture

```
User → TypeScript SDK → Rust CLI Binary → Rust Core
User → Rust CLI Binary → Rust Core (direct)
```

- **Rust Core**: Storage (RocksDB), Prolog parser (pest), WAM execution engine
- **Rust CLI**: JSON-over-STDIO interface
- **TypeScript SDK**: Calls Rust CLI via child_process

## Development

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p zlf-core
cargo test -p zlf-storage
cargo test -p zlf-query

# Build release binary
cargo build --release
```

## License

MIT OR Apache-2.0
