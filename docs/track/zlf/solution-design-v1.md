# Solution Design v1: zlf - AI-Native Graph Database with Logic Reasoning

## Source Requirements

- PRD / track: docs/track/zlf/prd-v1.md
- Requirement IDs: REQ-001 to REQ-008

## Design Principles

1. **Simplicity First**: Start with minimal viable architecture, add complexity only when needed
2. **Contract-First**: Define interfaces before implementation
3. **Layered Architecture**: Clear separation between storage, query engine, and API layers
4. **AI-Native**: Design for AI agent consumption from day one
5. **Performance by Default**: Embedded KV store + WAM for efficient execution

## Alternatives

| Option | Summary | Pros | Cons | Decision |
|---|---|---|---|---|
| A: Monolith in Rust | Single Rust binary with all features | Maximum performance, single deployment | Large codebase, harder to extend | ❌ |
| B: Rust Core + TypeScript Shell | Rust engine + TypeScript CLI/SDK via napi-rs | Performance + ecosystem, clear separation | FFI overhead, two languages | ✅ |
| C: Pure TypeScript | Everything in TypeScript | Simple stack, fast development | Performance limitations for graph algorithms | ❌ |

## Recommended Solution

**Option B: Rust Core + TypeScript Shell**

Rust handles ALL core logic:
- Storage engine (RocksDB wrapper)
- Prolog parser and lexer (pest/lalrpop)
- WAM execution engine (complete Prolog inference)
- Index management (temporal, BM25, vector)
- Query planning and execution

TypeScript handles ONLY user-facing wrapper:
- CLI (REPL + one-shot) - thin wrapper
- SDK for AI agents - thin wrapper
- LLM/embedding API calls - integration layer
- Configuration management

## Architecture / Module Landing

```
zlf/
├── crates/                    # ALL core logic in Rust
│   ├── zlf-core/              # Core types and traits
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── node.rs        # Node types and properties
│   │   │   ├── edge.rs        # Edge types and properties
│   │   │   ├── graph.rs       # Graph operations
│   │   │   └── error.rs       # Error types
│   │   └── Cargo.toml
│   │
│   ├── zlf-storage/           # RocksDB storage engine
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── kv.rs          # KV store operations
│   │   │   ├── wal.rs         # WAL management
│   │   │   ├── version.rs     # Node versioning
│   │   │   ├── overflow.rs    # Large property storage
│   │   │   └── cache.rs       # Fact caching
│   │   └── Cargo.toml
│   │
│   ├── zlf-index/             # Index management
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── temporal.rs    # Time index
│   │   │   ├── bm25.rs        # BM25 inverted index
│   │   │   └── vector.rs      # Vector index (pluggable)
│   │   └── Cargo.toml
│   │
│   ├── zlf-prolog/            # Prolog engine (complete)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── lexer.rs       # Prolog lexer
│   │   │   ├── parser.rs      # Prolog parser (pest/lalrpop)
│   │   │   ├── ast.rs         # AST types
│   │   │   ├── wam.rs         # Warren Abstract Machine
│   │   │   ├── unification.rs # Unification algorithm
│   │   │   └── builtin.rs     # Built-in predicates
│   │   └── Cargo.toml
│   │
│   ├── zlf-query/             # Query execution
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── planner.rs     # Query planner
│   │   │   ├── executor.rs    # Query executor
│   │   │   └── optimizer.rs   # Query optimizer
│   │   └── Cargo.toml
│   │
│   ├── zlf-api/               # API layer (Rust library)
│   │   ├── src/
│   │   │   └── lib.rs         # ZLF struct with JSON conversion
│   │   └── Cargo.toml
│   │
│   └── zlf-cli/               # CLI binary (JSON-over-STDIO)
│       ├── src/
│       │   └── main.rs        # JSON command handler
│       ├── tests/
│       │   └── integration_test.rs
│       └── Cargo.toml
│
├── packages/                  # TypeScript SDK (thin wrapper)
│   └── zlf/                   # Calls Rust CLI via child_process
│       ├── src/
│       │   ├── index.ts
│       │   ├── zlf.ts         # Main ZLF class
│       │   ├── types.ts       # TypeScript types
│       │   └── __tests__/
│       │       ├── zlf.test.ts        # Unit tests (mocked)
│       │       └── integration.test.ts # Integration tests
│       ├── package.json
│       ├── tsconfig.json
│       └── jest.config.js
│
├── docs/
│   └── track/zlf/            # Track documentation
│       ├── prd-v1.md
│       ├── solution-design-v1.md
│       ├── plan-v1.md
│       ├── delivery-record-v1.md
│       ├── change-note-001.md
│       └── review-feedback-001.md
│
└── Cargo.toml                 # Rust workspace
```

## Dependencies

### Core Dependencies (Rust)

| Crate | Version | License | Purpose |
|---|---|---|---|
| `rocksdb` | 0.24.x | Apache-2.0 | Embedded KV store |
| `pest` | 2.8.x | MIT/Apache-2.0 | PEG parser generator |
| `pest_derive` | 2.8.x | MIT/Apache-2.0 | Parser derive macro |
| `serde` | 1.x | MIT/Apache-2.0 | Serialization framework |
| `bincode` | 1.x | MIT | Binary serialization |
| `thiserror` | 2.x | MIT/Apache-2.0 | Error type derivation |
| `anyhow` | 1.x | MIT/Apache-2.0 | Application error handling |
| `tracing` | 0.1.x | MIT | Structured logging |
| `uuid` | 1.x | MIT/Apache-2.0 | UUID generation |
| `chrono` | 0.4.x | MIT/Apache-2.0 | Date/time handling |

### CLI Dependencies (Rust)

| Crate | Version | License | Purpose |
|---|---|---|---|
| `serde_json` | 1.x | MIT/Apache-2.0 | JSON serialization for STDIO |
| `anyhow` | 1.x | MIT/Apache-2.0 | Error handling |

### Search Dependencies (Rust)

| Crate | Version | License | Purpose |
|---|---|---|---|
| `tantivy` | 0.22.x | MIT | Full-text search (BM25) |
| `jieba-rs` | 0.10.x | MIT | Chinese word segmentation |
| `cang-jie` | 0.1.x | - | Tantivy Chinese tokenizer |

### Dependencies to Avoid

| Crate | Reason |
|---|---|
| `qdrant` | Too heavy (full vector database) |
| Commercial vector DB bindings | License restrictions |
| `lalrpop` | Less flexible than pest |

## Contracts

### Data Schema (RocksDB)

```
# Node Storage (current version only)
key: node:{uuid}
value: bincode(Node)
  - id: String
  - labels: Vec<String>
  - properties: HashMap<String, Value>
  - current_version: u64

# Edge Storage
key: edge:{uuid}
value: bincode(Edge)
  - id: String
  - edge_type: String
  - source: String
  - target: String
  - properties: HashMap<String, Value>

# Version History (separate key space)
key: ver:{node_uuid}:{version_id}
value: bincode(NodeVersion)
  - version_id: u64
  - properties: HashMap<String, Value>
  - valid_from: Timestamp
  - valid_to: Option<Timestamp>

# Index: Label -> Nodes
key: idx:label:{label}:{node_uuid}
value: ()

# Index: Edge Type -> Edges
key: idx:edge_type:{type}:{edge_uuid}
value: ()

# Index: Temporal
key: idx:temporal:{timestamp}:{node_uuid}
value: ()

# Index: BM25 (inverted)
key: idx:bm25:{token}:{node_uuid}
value: f32 (tf-idf score)

# Note: For Chinese text, tokens are generated using jieba-rs word segmentation
# Example: "我们中出了一个叛徒" -> ["我们", "中", "出", "了", "一个", "叛徒"]

# Overflow Storage
key: overflow:{uuid}
value: bytes
```

### API (TypeScript SDK)

```typescript
class ZLF {
  // Core operations
  addNode(labels: string[], properties: Record<string, any>): Node
  addEdge(type: string, source: string, target: string, properties?: Record<string, any>): Edge
  getNode(id: string): Node | null
  getEdge(id: string): Edge | null
  updateNode(id: string, properties: Record<string, any>): Node
  deleteNode(id: string): boolean
  deleteEdge(id: string): boolean

  // Query operations
  query(zlfLog: string): QueryResult[]
  queryWithPlan(zlfLog: string, options?: QueryOptions): QueryResult[]

  // Temporal operations
  getNodeVersions(id: string): NodeVersion[]
  getNodeAtTime(id: string, timestamp: Date): Node | null

  // Semantic search
  similar(nodeId: string, threshold: number, limit?: number): SimilarResult[]

  // BM25 search
  search(query: string, options?: SearchOptions): SearchResult[]

  // Memory operations (high-level)
  memory: MemoryManager
}

class MemoryManager {
  // Store memory with metadata
  store(id: string, data: {
    type: 'conversation' | 'knowledge' | 'task';
    content: Record<string, any>;
    entities?: string[];
    topics?: string[];
    importance?: number;  // 0-1, default 0.5
    ttl?: number;  // Time to live in seconds
  }): void

  // Retrieve memory by ID
  retrieve(id: string): MemoryData | null

  // Query memories by pattern
  query(pattern: {
    type?: string;
    entities?: string[];
    topics?: string[];
    timeRange?: { start: Date; end: Date };
    minImportance?: number;
  }): MemoryData[]

  // Update memory importance
  updateImportance(id: string, importance: number): void

  // Expire old memories
  expire(olderThan: Date): number  // Returns count of expired

  // Consolidate similar memories
  consolidate(threshold: number): number  // Returns count of consolidated
}
```

### CLI Interface (JSON-over-STDIO)

The CLI accepts JSON commands via STDIN and returns JSON responses via STDOUT.

**Request Format:**
```json
{"command": "<command>", "path": "<db-path>", ...params}
```

**Response Format:**
```json
{"type": "success", "data": {...}}
// or
{"type": "error", "code": "ERROR_CODE", "message": "description"}
```

**Supported Commands:**

| Command | Parameters | Description |
|---------|------------|-------------|
| `init` | `path` | Initialize database |
| `add_node` | `path`, `labels`, `properties` | Add a node |
| `get_node` | `path`, `id` | Get node by ID |
| `add_edge` | `path`, `edge_type`, `source`, `target`, `properties` | Add an edge |
| `get_edge` | `path`, `id` | Get edge by ID |
| `query` | `path`, `query` | Execute zlf-log query |
| `search` | `path`, `query` | BM25 search |
| `similar` | `path`, `node_id`, `threshold`, `limit` | Semantic search |

**Example Usage:**
```bash
# Initialize database
echo '{"command":"init","path":"./my-db"}' | zlf

# Add a node
echo '{"command":"add_node","path":"./my-db","labels":["person"],"properties":{"name":"Alice"}}' | zlf

# Query
echo '{"command":"query","path":"./my-db","query":"node(person, X, _)."}' | zlf
```

**TypeScript SDK Usage:**
```typescript
import { ZLF } from 'zlf';

const db = new ZLF('./my-db');
const node = await db.addNode(['person'], { name: 'Alice' });
const retrieved = await db.getNode(node.id);
```

## Query Execution Flow

```
TypeScript SDK
    ↓ child_process
Rust zlf-cli (JSON-over-STDIO)
    ↓
Rust zlf-api (JSON conversion)
    ↓
Rust zlf-query (planner + executor)
    ↓
Rust zlf-prolog (parser + WAM)
    ↓
Rust zlf-storage (KV operations)
    ↓
Rust zlf-index (index lookups)
    ↓
Results as JSON → TypeScript SDK
```

**Detailed Flow:**
1. TypeScript SDK serializes command as JSON
2. Rust CLI receives JSON via STDIN
3. Parse zlf-log query (Rust parser)
4. Generate WAM instructions (Rust WAM)
5. Plan execution order (Rust planner)
   - Graph traversal first
   - Then semantic filtering
   - Then temporal filtering
6. Execute WAM with dynamic facts (Rust)
   - Load nodes/edges from storage
   - Apply rules and backtracking
7. Apply filters (semantic, temporal)
8. Return results as JSON via STDOUT

## Test Strategy

### Unit Tests

**Parser Tests (zlf-prolog)**:
- Valid syntax: facts, rules, queries, aggregation
- Invalid syntax: missing dots, unclosed brackets, undefined predicates
- Edge cases: empty input, whitespace handling, special characters

**WAM Tests (zlf-prolog)**:
- Instruction execution: put, get, unify, call, proceed
- Choice points: backtracking, cut operation
- Unification: variable binding, term matching
- Built-in predicates: time, search, similar

**Storage Tests (zlf-storage)**:
- CRUD operations: create, read, update, delete nodes/edges
- Version management: create version, get history, time travel
- Overflow: large property storage and retrieval
- WAL: crash recovery, data integrity

**Index Tests (zlf-index)**:
- Temporal index: add, query by range, cleanup
- BM25 index: add, search, ranking
- Vector index: add, similarity search

### Integration Tests

**Query Execution**:
- Simple graph traversal
- Rule-based inference
- Combined graph + semantic + temporal queries
- Backtracking with multiple solutions

**End-to-End Flows**:
- Import JSON → Query → Export
- Create nodes → Define rules → Execute queries
- Store memory → Query by time → Expire old memories

**Error Scenarios**:
- Invalid input handling
- Concurrent access conflicts
- Storage quota exceeded
- Timeout handling

### E2E Tests

**CLI Binary (JSON-over-STDIO)**:
- Init database
- Add/get nodes
- Add/get edges
- Query execution
- Error handling (invalid JSON, missing DB, etc.)

**TypeScript SDK**:
- SDK operations (addNode, getNode, addEdge, etc.)
- Memory operations
- Error propagation
- Integration with Rust CLI binary

**Integration Tests**:
- Full flow: TypeScript SDK → Rust CLI → Database
- Edge cases (empty labels, nested properties, etc.)
- Unhappy paths (non-existent nodes, invalid paths, etc.)

### Manual Tests

**Performance**:
- 100K nodes query performance
- 1M edges traversal performance
- Memory usage under load

**Usability**:
- REPL interaction
- Error message clarity
- Documentation accuracy

## Rollback / Migration Strategy

- **Database Format**: Use versioned file format, support migration scripts
- **API Changes**: Semantic versioning, deprecation warnings
- **Query Language**: Backward compatible extensions only

## Open Design Questions

- [ ] How to handle embedding model updates?
- [ ] What's the maximum property size before overflow?
- [ ] How to implement efficient graph traversal for large graphs?

## Design Answers

### Q1: How to handle embedding model updates?

**Answer**: Each embedding stores its model name and version. When querying, users can specify which embedding to use. Multiple embedding versions can coexist. Model updates create new embeddings, old ones remain queryable.

### Q2: What's the maximum property size before overflow?

**Answer**: Properties larger than 1KB are automatically stored in overflow storage (separate KV entries). The main node/edge record stores a reference (UUID) to the overflow data. This threshold is configurable.

### Q3: How to implement efficient graph traversal for large graphs?

**Answer**: 
1. **Index-first approach**: Always use indexes (label, edge type) before full scans
2. **Fact caching**: Cache frequently accessed nodes/edges in memory
3. **Lazy loading**: Only load node properties when accessed
4. **Depth limits**: Default max depth for recursive rules (configurable)
