# zlf Usage Guide

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [CLI Reference](#cli-reference)
4. [TypeScript SDK](#typescript-sdk)
5. [Query Language](#query-language)
6. [Examples](#examples)

## Installation

### From Source

```bash
git clone <repository-url>
cd zlf
cargo build --release
```

The binary will be at `target/release/zlf`.

### Using npm (TypeScript SDK)

```bash
npm install zlf
```

## Quick Start

### 1. Initialize a Database

```bash
echo '{"command":"init","path":"./my-graph.db"}' | ./target/release/zlf
```

### 2. Add Nodes

```bash
# Add a person
echo '{"command":"add_node","path":"./my-graph.db","labels":["person"],"properties":{"name":"Alice","age":30,"city":"Beijing"}}' | ./target/release/zlf

# Add a company
echo '{"command":"add_node","path":"./my-graph.db","labels":["company"],"properties":{"name":"ACME","industry":"Tech"}}' | ./target/release/zlf
```

### 3. Add Edges

```bash
# First, get the node IDs from the add_node responses
# Then add an edge
echo '{"command":"add_edge","path":"./my-graph.db","edge_type":"works_at","source":"<alice-id>","target":"<acme-id>","properties":{"since":2020}}' | ./target/release/zlf
```

### 4. Query Data

```bash
# Query all persons
echo '{"command":"query","path":"./my-graph.db","query":"node(person, X, Props)."}' | ./target/release/zlf

# Query edges
echo '{"command":"query","path":"./my-graph.db","query":"edge(works_at, X, Y, Props)."}' | ./target/release/zlf
```

### 5. Search

```bash
# BM25 text search
echo '{"command":"search","path":"./my-graph.db","query":"software engineer"}' | ./target/release/zlf

# Semantic search (requires embeddings)
echo '{"command":"similar","path":"./my-graph.db","node_id":"<node-id>","threshold":0.8,"limit":10}' | ./target/release/zlf
```

## CLI Reference

### Request Format

```json
{
  "command": "<command>",
  "path": "<database-path>",
  ...additional-params
}
```

### Response Format

```json
// Success
{"type": "success", "data": {...}}

// Error
{"type": "error", "code": "ERROR_CODE", "message": "description"}
```

### Commands

#### init

Initialize a new database.

```json
{"command": "init", "path": "./my-db"}
```

#### add_node

Add a node with labels and properties.

```json
{
  "command": "add_node",
  "path": "./my-db",
  "labels": ["person", "employee"],
  "properties": {"name": "Alice", "age": 30}
}
```

#### get_node

Get a node by ID.

```json
{"command": "get_node", "path": "./my-db", "id": "<node-id>"}
```

#### add_edge

Add an edge between two nodes.

```json
{
  "command": "add_edge",
  "path": "./my-db",
  "edge_type": "knows",
  "source": "<source-node-id>",
  "target": "<target-node-id>",
  "properties": {"since": 2020}
}
```

#### get_edge

Get an edge by ID.

```json
{"command": "get_edge", "path": "./my-db", "id": "<edge-id>"}
```

#### query

Execute a zlf-log query.

```json
{"command": "query", "path": "./my-db", "query": "node(person, X, Props)."}
```

#### search

BM25 full-text search.

```json
{"command": "search", "path": "./my-db", "query": "software engineer"}
```

#### similar

Semantic similarity search.

```json
{
  "command": "similar",
  "path": "./my-db",
  "node_id": "<node-id>",
  "threshold": 0.8,
  "limit": 10
}
```

#### import

Import data from a JSON file.

```json
{"command": "import", "path": "./my-db", "file": "./data.json"}
```

Import file format:
```json
{
  "nodes": [
    {"labels": ["person"], "properties": {"name": "Alice"}},
    {"labels": ["person"], "properties": {"name": "Bob"}}
  ],
  "edges": [
    {"edge_type": "knows", "source": "<alice-id>", "target": "<bob-id>", "properties": {}}
  ]
}
```

#### export

Export data to JSON.

```json
{"command": "export", "path": "./my-db", "file": "./output.json"}
```

## TypeScript SDK

### Installation

```bash
npm install zlf
```

### Usage

```typescript
import { ZLF } from 'zlf';

// Initialize
const db = new ZLF('./my-db');

// Add node
const node = await db.addNode(['person'], { name: 'Alice', age: 30 });
console.log(node.id); // UUID

// Get node
const retrieved = await db.getNode(node.id);

// Add edge
const bob = await db.addNode(['person'], { name: 'Bob' });
const edge = await db.addEdge('knows', node.id, bob.id, { since: 2020 });

// Query
const results = await db.query('node(person, X, Props).');

// Search
const searchResults = await db.search('software engineer');

// Similarity search
const similar = await db.similar(node.id, 0.8, 10);

// Memory operations
await db.storeMemory('conv123', {
  type: 'conversation',
  content: { message: 'Hello World' },
  entities: ['alice', 'bob'],
  importance: 0.9
});

const memory = await db.getMemory('conv123');
```

## Query Language

zlf-log is a Prolog-style query language.

### Facts

```prolog
% Query nodes by label
node(person, X, Props).

% Query edges by type
edge(knows, X, Y, Props).
```

### Rules

```prolog
% Define a rule
colleague(X, Y) :- works_at(X, C), works_at(Y, C), X \= Y.

% Use the rule
?colleague(alice, Who).
```

### Built-in Predicates

- `node(Label, Id, Props)` - Query nodes
- `edge(Type, Source, Target, Props)` - Query edges
- `search(Query)` - BM25 search
- `similar_to(NodeId, Threshold, Results)` - Semantic search

## Examples

### Knowledge Graph

```bash
# Initialize
echo '{"command":"init","path":"./knowledge"}' | zlf

# Add concepts
echo '{"command":"add_node","path":"./knowledge","labels":["concept"],"properties":{"name":"Machine Learning"}}' | zlf
echo '{"command":"add_node","path":"./knowledge","labels":["concept"],"properties":{"name":"Deep Learning"}}' | zlf

# Add relationship
echo '{"command":"add_edge","path":"./knowledge","edge_type":"is_subset_of","source":"<dl-id>","target":"<ml-id>","properties":{}}' | zlf
```

### Agent Memory

```typescript
import { ZLF } from 'zlf';

const db = new ZLF('./agent-memory');

// Store conversation memory
await db.storeMemory('conv-001', {
  type: 'conversation',
  content: { 
    user: "What's the weather?",
    assistant: "It's sunny today."
  },
  entities: ['weather'],
  topics: ['weather', 'daily'],
  importance: 0.3
});

// Store knowledge memory
await db.storeMemory('know-001', {
  type: 'knowledge',
  content: {
    fact: "Python is a programming language",
    source: "documentation"
  },
  entities: ['python'],
  topics: ['programming', 'languages'],
  importance: 0.8
});

// Query memories
const weatherMemories = await db.queryMemories({ type: 'conversation' });
```

## Error Handling

All errors return a structured response:

```json
{
  "type": "error",
  "code": "NODE_NOT_FOUND",
  "message": "Node with ID 'abc123' not found"
}
```

### Error Codes

| Code | Description |
|------|-------------|
| `INIT_FAILED` | Failed to initialize database |
| `DB_OPEN_FAILED` | Database not found or cannot be opened |
| `ADD_NODE_FAILED` | Failed to add node |
| `NODE_NOT_FOUND` | Node with given ID not found |
| `ADD_EDGE_FAILED` | Failed to add edge |
| `EDGE_NOT_FOUND` | Edge with given ID not found |
| `QUERY_FAILED` | Query execution failed |
| `SEARCH_FAILED` | Search failed |
| `IMPORT_FAILED` | Import failed |
| `EXPORT_FAILED` | Export failed |
| `INVALID_REQUEST` | Invalid JSON request |
