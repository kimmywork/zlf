# PRD v1: zlf - AI-Native Graph Database with Logic Reasoning

## 0. Metadata

- Owner: kimmy
- Status: draft
- Last updated: 2026-07-06

## 1. Elevator Pitch

zlf: A lightweight graph database that combines property graph storage with Prolog-style logic reasoning, designed specifically for AI agents and knowledge management. Unlike traditional graph databases that focus on traversal, zlf adds declarative rule-based inference and backtracking to simplify complex relationship queries.

## 2. Background / Problem

- **Current pain**: 
  - Existing graph databases (Neo4j, TigerGraph) are powerful but complex to set up and query
  - Vector + Graph solutions (GraphRAG) only store relationships, don't enable reasoning
  - AI agents need rich relationship queries but current APIs are not agent-friendly
  - Complex multi-hop queries require verbose traversal code

- **Why now**:
  - AI agents are becoming mainstream, need persistent memory with relationships
  - Knowledge management needs semantic connections, not just keyword search
  - Prolog-style logic can dramatically simplify graph queries

- **Existing constraints**:
  - Must be lightweight (single-node, no cluster dependency)
  - Must support CLI for immediate usability
  - Must be extensible (MCP can come later)

## 3. User Persona

| Persona | Situation | Need | Constraint |
|---|---|---|---|
| Knowledge Worker | Manages complex concept relationships in team (几十万节点) | Query relationships semantically, discover connections | Simple interface, fast queries, multi-user read |
| AI Agent Developer | Builds agents with unified memory system | Store/retrieve conversation, knowledge, task memory with context | Agent-friendly API, temporal support, auto-expire + importance ranking |
| Researcher | Explores knowledge graphs | Run complex inference queries | Declarative query language |

## 3.1 Scale & Usage Patterns

- **Knowledge Base**: Team-scale, 几十万节点, multi-user read access
- **Agent Memory**: Unified system covering:
  - Conversation memory (cross-session persistence)
  - Knowledge memory (concepts + relationships)
  - Task memory (status + progress tracking)
- **Memory Lifecycle**: Auto-expire + importance ranking + user marking
- **Query Patterns**: Temporal, Entity-based, Semantic search, Relationship traversal, Composite
- **Integration**: CLI + SDK (HTTP service deferred to v2)
- **Schema Strategy**: Schema-free (add node types freely, infer later)
- **Inference Level**: Rule-based inference (Prolog rules for implicit relationship derivation)
- **Data Exchange**: JSON + Auto-parse Markdown/documents

## 4. Business / Value Canvas

- **User value**: Simplified graph queries through logic reasoning, better AI memory management
- **Project value**: Foundation for AI-native knowledge systems
- **Success signal**: Complex 5-hop queries written in <10 lines of declarative rules
- **Adoption risk**: Query language learning curve
- **Cost/risk**: Initial development complexity

## 5. Query Language Design: zlf-log

### Core Concept: Prolog + Graph + Semantic + Temporal

**Traditional Cypher** (verbose):
```cypher
MATCH (a:Person)-[:KNOWS]->(b:Person)-[:WORKS_AT]->(c:Company)
WHERE a.name = 'Alice' AND c.industry = 'Tech'
RETURN b.name
```

**zlf-log** (declarative rules):
```prolog
% Define a rule
colleague_of(X, Y) :- knows(X, Y), works_at(Y, Company), company_industry(Company, tech).

% Query
?colleague_of(alice, Z).
```

### Language Features

#### 1. Facts (Data)
```prolog
% Node with properties
node(person, alice, {name: "Alice", age: 30}).
node(company, acme, {name: "ACME", industry: "Tech"}).

% Edges with properties
edge(alice, knows, bob, {since: 2020}).
edge(bob, works_at, acme, {role: "Engineer"}).
```

#### 2. Rules (Inference)
```prolog
% Simple rule
colleague(X, Y) :- works_at(X, C), works_at(Y, C), X \= Y.

% Recursive rule (variable-length paths)
ancestor(X, Y) :- parent(X, Y).
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

% Conditional rule
senior(X) :- age(X, A), A >= 50.
```

#### 3. Queries (Backtracking)
```prolog
% Find all colleagues of Alice
?colleague(alice, Who).

% Find all ancestors
?ancestor(alice, Who).

% Find with conditions
?colleague(alice, Y), works_at(Y, C), industry(C, tech).
```

#### 4. Aggregation
```prolog
% Count
?count(colleague(alice, _), N).

% Group by
?group_by(works_at(_, C), C, Count), count(_, Count).
```

#### 5. Semantic Search
```prolog
% Find similar nodes by embedding
?similar_to(alice, 0.8, Results).

% Combine with graph traversal
?similar_to(alice, 0.8, X), works_at(X, C).
```

#### 6. Temporal Queries
```prolog
% Time range filter
?node(person, X, _), time_range(X, last_week).

% Before/after
?node(person, X, _), after(X, '2026-01-01').

% Between
?node(person, X, _), between(X, '2026-01-01', '2026-06-30').
```

#### 7. BM25 Search
```prolog
% Full-text search on properties
?search(person, 'Alice Smith', Results).

% Combine with graph traversal
?search(person, 'engineer', X), works_at(X, C).
```

## 6. Data Model

### Property Graph with Types
```
┌─────────────────────────────────────────────────────────┐
│                       zlf                               │
├─────────────────────────────────────────────────────────┤
│  Nodes                                                  │
│  ├─ id: string (unique)                                │
│  ├─ labels: [string] (type system)                     │
│  └─ properties: {key: value}                           │
│                                                         │
│  Edges                                                  │
│  ├─ id: string (unique)                                │
│  ├─ type: string (relationship type)                   │
│  ├─ source: node_id                                    │
│  ├─ target: node_id                                    │
│  ├─ direction: directed | undirected                   │
│  └─ properties: {key: value}                           │
│                                                         │
│  Temporal Layer (Node Versioning)                        │
│  ├─ Each node has version history                        │
│  ├─ Edges reference specific node versions               │
│  ├─ valid_from: timestamp (per version)                  │
│  └─ valid_to: timestamp (per version)                    │
│                                                         │
│  Vector Layer (Optional)                                │
│  ├─ embedding: float[] (for semantic search)           │
│  └─ embedding_model: string                            │
└─────────────────────────────────────────────────────────┘
```

## 7. API Design

### CLI Interface (JSON-over-STDIO)

The CLI accepts JSON commands via STDIN and returns JSON via STDOUT.

```bash
# Initialize database
echo '{"command":"init","path":"./my-graph.db"}' | zlf

# Add a node
echo '{"command":"add_node","path":"./my-graph.db","labels":["person"],"properties":{"name":"Alice","age":30}}' | zlf

# Get a node
echo '{"command":"get_node","path":"./my-graph.db","id":"<node-id>"}' | zlf

# Add an edge
echo '{"command":"add_edge","path":"./my-graph.db","edge_type":"knows","source":"<source-id>","target":"<target-id>","properties":{"since":2020}}' | zlf

# Execute query
echo '{"command":"query","path":"./my-graph.db","query":"node(person, X, _)."}' | zlf

# BM25 search
echo '{"command":"search","path":"./my-graph.db","query":"software engineer"}' | zlf

# Semantic search
echo '{"command":"similar","path":"./my-graph.db","node_id":"<node-id>","threshold":0.8,"limit":10}' | zlf
```

**Response Format:**
```json
{"type": "success", "data": {...}}
// or
{"type": "error", "code": "ERROR_CODE", "message": "description"}
```

### Programmatic API (TypeScript SDK)
```typescript
import { ZLF } from 'zlf';

const db = new ZLF('./my-graph.db');

// Add a node
const node = await db.addNode(['person'], { name: 'Alice', age: 30 });

// Get a node
const retrieved = await db.getNode(node.id);

// Add an edge
const edge = await db.addEdge('knows', node.id, bobNode.id, { since: 2020 });

// Execute query
const results = await db.query('node(person, X, _).');

// BM25 search
const searchResults = await db.search('software engineer');

// Semantic search
const similarResults = await db.similar(node.id, 0.8, 10);

// Memory operations
await db.storeMemory('conv123', {
  type: 'conversation',
  content: { message: 'Hello' },
  entities: ['alice', 'bob'],
  importance: 0.8
});

const memory = await db.getMemory('conv123');
```

## 8. Scope

- **In scope**:
  - Core graph storage (nodes, edges, properties)
  - zlf-log query language (facts, rules, queries, backtracking)
  - CLI interface (JSON-over-STDIO)
  - Temporal layer (node versioning)
  - TypeScript SDK (calls Rust CLI via child_process)
  - Pluggable embedding for semantic search
  - BM25 full-text search
  - Import/export (JSON)

- **Out of scope**:
  - MCP integration (future)
  - Distributed/cluster mode
  - Graph algorithms (BFS, DFS, PageRank, etc.)
  - Visualization UI

## 9. Non-Goals

| ID | Non-Goal | Reason |
|---|---|---|
| NG1 | Replace Neo4j | Focus on AI-native use cases, not general-purpose |
| NG2 | Real-time streaming | Batch-oriented for knowledge management |
| NG3 | Full ACID transactions | Single-user/agent focus; WAL provides crash recovery, not full transactional isolation |

## 10. Requirements

### REQ-001: Node Storage

**Description**: System shall store nodes with labels and properties.

**Acceptance Criteria (Given/When/Then)**:
```
Given an empty database
When user creates a node with labels ["person"] and properties {"name": "Alice", "age": 30}
Then the node is persisted with:
  - Unique ID (UUID format)
  - Labels: ["person"]
  - Properties: {"name": "Alice", "age": 30}
  - Created timestamp
  - Version: 1
```

**Edge Cases**:
- EC-001.1: Node with empty labels array → System shall accept (labels are optional)
- EC-001.2: Node with empty properties object → System shall accept
- EC-001.3: Node with nested properties → System shall serialize and store
- EC-001.4: Node with very large properties (>1KB) → System shall use overflow storage

**Unhappy Paths**:
- UP-001.1: Duplicate node ID → System shall return error "Node with ID {id} already exists"
- UP-001.2: Invalid property value type → System shall return error "Invalid property value for key {key}"
- UP-001.3: Node ID exceeds max length (255 chars) → System shall return error "Node ID too long"

---

### REQ-002: Edge Storage

**Description**: System shall store edges with type, source, target, and properties.

**Acceptance Criteria**:
```
Given nodes "alice" and "bob" exist in the database
When user creates an edge with type "knows", source "alice", target "bob", and properties {"since": 2020}
Then the edge is persisted with:
  - Unique ID (UUID format)
  - Type: "knows"
  - Source: "alice"
  - Target: "bob"
  - Properties: {"since": 2020}
  - Created timestamp
```

**Edge Cases**:
- EC-002.1: Edge with no properties → System shall accept (properties default to {})
- EC-002.2: Self-referencing edge (source = target) → System shall accept
- EC-002.3: Multiple edges between same nodes → System shall accept (multi-graph)

**Unhappy Paths**:
- UP-002.1: Source node does not exist → System shall return error "Source node {source} not found"
- UP-002.2: Target node does not exist → System shall return error "Target node {target} not found"
- UP-002.3: Edge type is empty → System shall return error "Edge type cannot be empty"

---

### REQ-003: Rule Compilation

**Description**: System shall compile Prolog rules for evaluation.

**Acceptance Criteria**:
```
Given a valid Prolog rule:
  colleague(X, Y) :- works_at(X, C), works_at(Y, C), X \= Y.
When user submits the rule
Then the rule is compiled and stored for query execution
```

**Edge Cases**:
- EC-003.1: Rule with recursive definition → System shall accept (with depth limit)
- EC-003.2: Rule with multiple clauses → System shall accept
- EC-003.3: Rule with built-in predicates → System shall accept

**Unhappy Paths**:
- UP-003.1: Invalid Prolog syntax → System shall return error with line number and suggestion
- UP-003.2: Undefined predicate in rule → System shall return error "Predicate {name} not defined"
- UP-003.3: Rule exceeds max complexity (100 clauses) → System shall return error "Rule too complex"

---

### REQ-004: Query Execution with Backtracking

**Description**: System shall execute queries and return all matching solutions via backtracking.

**Acceptance Criteria**:
```
Given nodes: alice (person), bob (person), charlie (person), acme (company)
Given edges: alice --knows--> bob, bob --knows--> charlie, bob --works_at--> acme, charlie --works_at--> acme
Given rule: colleague(X, Y) :- works_at(X, C), works_at(Y, C), X \= Y.
When user executes query: ?colleague(alice, Who).
Then system returns all solutions:
  - [] (alice has no colleagues - doesn't work at acme)
When user executes query: ?colleague(bob, Who).
Then system returns all solutions:
  - [charlie] (bob and charlie both work at acme)
When user executes query: ?colleague(Who, bob).
Then system returns all solutions:
  - [charlie] (charlie and bob both work at acme)
```

**Edge Cases**:
- EC-004.1: Query with no matching results → System shall return empty result set
- EC-004.2: Query with infinite recursion → System shall enforce depth limit (default: 100) and return partial results
- EC-004.3: Query combining graph, semantic, and temporal → System shall execute in order: graph → semantic → time
- EC-004.4: Query with multiple rules → System shall evaluate all rules and combine results
- EC-004.5: Query with variable binding across clauses → System shall maintain binding context
- EC-004.6: Query with negation (\=) → System shall correctly handle inequality checks

**Unhappy Paths**:
- UP-004.1: Query references undefined predicate → System shall return error "Predicate {name} not defined"
- UP-004.2: Query exceeds max execution time (30s) → System shall timeout and return error "Query timeout after 30s"
- UP-004.3: Query exceeds max result count (10000) → System shall truncate and return warning "Results truncated to 10000"
- UP-004.4: Query has syntax error → System shall return error with line number and suggestion
- UP-004.5: Query uses unsupported feature → System shall return error "Feature not supported: {feature}"

---

### REQ-005: Temporal Queries

**Description**: System shall support time-range filters for queries.

**Acceptance Criteria**:
```
Given a node with versions:
  - Version 1: valid_from=2026-01-01, valid_to=2026-06-30
  - Version 2: valid_from=2026-07-01, valid_to=null
When user executes query: ?node(person, X, _), time_range(X, last_week).
Then system returns nodes valid during last week
```

**Edge Cases**:
- EC-005.1: Node with single version → System shall return if within time range
- EC-005.2: Node with overlapping versions → System shall return latest valid version
- EC-005.3: Query with invalid date format → System shall return error

**Unhappy Paths**:
- UP-005.1: Time index corrupted → System shall rebuild index on startup
- UP-005.2: Query with future timestamp → System shall return current version

---

### REQ-006: Agent Memory Storage

**Description**: System shall create temporal graph entries for agent memory.

**Acceptance Criteria**:
```
Given an agent with ID "agent-1"
When agent stores memory with:
  - type: "conversation"
  - content: {"message": "Hello"}
  - entities: ["alice"]
  - importance: 0.8
Then memory is stored as a node with:
  - Labels: ["memory", "conversation"]
  - Properties: content + entities + importance
  - Temporal version created
  - Expiry timestamp calculated (based on importance)
```

**Edge Cases**:
- EC-006.1: Memory with no entities → System shall accept
- EC-006.2: Memory with importance > 1.0 → System shall clamp to 1.0
- EC-006.3: Memory with TTL = 0 → System shall expire immediately

**Unhappy Paths**:
- UP-006.1: Storage full → System shall return error "Storage quota exceeded"
- UP-006.2: Invalid memory type → System shall return error "Invalid memory type: {type}"

---

### REQ-007: Semantic Search

**Description**: System shall find similar nodes by embedding.

**Acceptance Criteria**:
```
Given nodes with embeddings (dimension: 3):
  - alice: [0.1, 0.2, 0.3]
  - bob: [0.15, 0.25, 0.35]
  - acme: [0.9, 0.8, 0.7]
When user executes query: ?similar_to(alice, 0.8, Results).
Then system returns nodes with cosine similarity > 0.8:
  - bob (similarity: 0.95)
When user executes query: ?similar_to(alice, 0.5, Results).
Then system returns nodes with cosine similarity > 0.5:
  - bob (similarity: 0.95)
  - acme (similarity: 0.6) - if above threshold
```

**Embedding Provider Interface**:
```typescript
interface EmbeddingProvider {
  // Generate embedding for text
  embed(text: string): Promise<number[]>;
  
  // Get embedding dimension
  dimension(): number;
  
  // Check if provider is available
  isAvailable(): Promise<boolean>;
}

// Supported providers:
// - OpenAI: text-embedding-3-small, text-embedding-3-large
// - Local: ONNX models, sentence-transformers
// - Custom: User-provided API
```

**Edge Cases**:
- EC-007.1: No nodes with embeddings → System shall return empty result
- EC-007.2: Node with multiple embedding versions → System shall use latest (by timestamp)
- EC-007.3: Threshold = 0.0 → System shall return all nodes with embeddings
- EC-007.4: Threshold = 1.0 → System shall return only exact matches
- EC-007.5: Query node has no embedding → System shall return error "Node {id} has no embedding"
- EC-007.6: Different embedding models → System shall only compare same-dimension embeddings

**Unhappy Paths**:
- UP-007.1: Embedding dimension mismatch → System shall return error "Dimension mismatch: expected {expected}, got {actual}"
- UP-007.2: Embedding provider unavailable → System shall return error "Embedding service unavailable: {provider}"
- UP-007.3: Embedding API rate limit → System shall retry with exponential backoff (max 3 retries)
- UP-007.4: Embedding generation timeout → System shall return error "Embedding timeout after 10s"
- UP-007.5: Invalid embedding values (NaN, Inf) → System shall return error "Invalid embedding values"

---

### REQ-008: BM25 Search

**Description**: System shall support BM25 full-text search with Chinese text processing.

**Acceptance Criteria**:
```
Given nodes with text properties (including Chinese):
  - alice: {"name": "Alice Smith", "bio": "Software engineer"}
  - bob: {"name": "张三", "bio": "软件工程师"}
When user executes query: ?search(person, 'engineer', Results).
Then system returns nodes ranked by BM25 score:
  - alice (score: 2.5)
When user executes query: ?search(person, '软件', Results).
Then system returns nodes ranked by BM25 score:
  - bob (score: 2.5)
```

**Chinese Text Processing**:
```
Input: "我们中出了一个叛徒"
Tokenization: ["我们", "中", "出", "了", "一个", "叛徒"]
Indexing: Each token creates inverted index entry
Search: Query tokenized the same way before matching
```

**Edge Cases**:
- EC-008.1: Empty search query → System shall return error
- EC-008.2: Search with special characters → System shall escape properly
- EC-008.3: Search across multiple property types → System shall search all text properties
- EC-008.4: Mixed Chinese and English text → System shall handle both languages
- EC-008.5: Chinese punctuation → System shall ignore or treat as delimiter

**Unhappy Paths**:
- UP-008.1: BM25 index not built → System shall build index on first search
- UP-008.2: Search timeout → System shall return partial results with warning
- UP-008.3: Jieba segmentation failure → System shall fallback to character-level tokenization

---

### REQ-009: Node Versioning

**Description**: System shall maintain version history for nodes.

**Acceptance Criteria**:
```
Given node "alice" with version 1
When user updates node properties
Then:
  - New version (version 2) is created
  - Version 1 valid_to is set to current timestamp
  - Version 2 valid_from is set to current timestamp
  - Node current_version is updated to 2
```

**Edge Cases**:
- EC-009.1: Update with same properties → System shall still create new version
- EC-009.2: Concurrent updates from multiple agents → System shall use optimistic locking

**Unhappy Paths**:
- UP-009.1: Version conflict → System shall return error with conflict details
- UP-009.2: Max versions exceeded (1000) → System shall return error

---

### REQ-010: Import/Export

**Description**: System shall support JSON import/export and document auto-parse.

**Acceptance Criteria**:
```
Given a JSON file with nodes and edges
When user executes: zlf import data.json
Then all nodes and edges are imported with correct relationships
```

**Edge Cases**:
- EC-010.1: Import with duplicate IDs → System shall skip duplicates with warning
- EC-010.2: Import with invalid references → System shall skip invalid edges with warning
- EC-010.3: Export empty database → System shall export empty JSON

**Unhappy Paths**:
- UP-010.1: Invalid JSON format → System shall return error "Invalid JSON: {details}"
- UP-010.2: File not found → System shall return error "File not found: {path}"
- UP-010.3: Permission denied → System shall return error "Permission denied: {path}"

---

### REQ-011: Database Management

**Description**: System shall support init, status, backup, and restore operations.

**Acceptance Criteria**:
```
When user executes: zlf init ./my-db
Then a new database is created at ./my-db with default configuration
```

**Edge Cases**:
- EC-011.1: Init existing database → System shall return error "Database already exists"
- EC-011.2: Backup to existing file → System shall overwrite with warning
- EC-011.3: Restore to non-empty database → System shall require confirmation

**Unhappy Paths**:
- UP-011.1: Disk full → System shall return error "Insufficient disk space"
- UP-011.2: Corrupted backup → System shall return error "Backup integrity check failed"

---

### REQ-012: Error Handling

**Description**: System shall provide detailed error messages with context.

**Error Code System**:
```
Error Code Format: {DOMAIN}_{ERROR_TYPE}
Examples:
  - NODE_NOT_FOUND
  - EDGE_INVALID_SOURCE
  - QUERY_TIMEOUT
  - STORAGE_QUOTA_EXCEEDED
  - PARSER_SYNTAX_ERROR

Domain Prefixes:
  - NODE: Node operations
  - EDGE: Edge operations
  - QUERY: Query execution
  - RULE: Rule compilation
  - STORAGE: Storage operations
  - INDEX: Index operations
  - IMPORT: Import operations
  - EXPORT: Export operations
  - CONFIG: Configuration
  - EMBEDDING: Embedding operations
```

**Acceptance Criteria**:
```
When any error occurs
Then system returns JSON error response:
{
  "error": {
    "code": "NODE_NOT_FOUND",
    "message": "Node with ID 'alice' not found",
    "suggestion": "Check if the node exists using 'zlf node get alice'",
    "context": {
      "operation": "create_edge",
      "node_id": "alice",
      "timestamp": "2026-07-06T10:00:00Z"
    },
    "stack_trace": "..." // only in debug mode
  }
}
```

**Error Response Contract**:
```typescript
interface ErrorResponse {
  error: {
    code: string;           // Error code (e.g., "NODE_NOT_FOUND")
    message: string;        // Human-readable message
    suggestion?: string;    // How to fix (when applicable)
    context?: Record<string, any>;  // Operation context
    stack_trace?: string;   // Only in debug mode
  };
}
```

**Edge Cases**:
- EC-012.1: Error in nested operation → System shall include full context chain
- EC-012.2: Error during transaction → System shall rollback and report
- EC-012.3: Multiple errors in batch operation → System shall return all errors with indices
- EC-012.4: Error in async operation → System shall return error with operation ID

**Unhappy Paths**:
- UP-012.1: Error logging fails → System shall write to stderr and continue
- UP-012.2: Error causes panic → System shall catch panic and return structured error
- UP-012.3: Error response too large → System shall truncate context and add warning
- UP-012.4: Error in error handler → System shall return minimal error response

---

### REQ-013: TypeScript SDK (FFI Integration)

**Description**: System shall provide TypeScript SDK via napi-rs FFI bindings.

**Acceptance Criteria**:
```
Given Rust zlf-api crate with FFI bindings
When TypeScript imports ZLF class
Then:
  - addNode(labels, properties) returns Node object
  - addEdge(type, source, target, properties) returns Edge object
  - getNode(id) returns Node | null
  - query(zlfLog) returns QueryResult[]
  - All operations throw typed errors on failure
```

**Error Handling Contract**:
```typescript
class ZLFError extends Error {
  code: string;      // e.g., "NODE_NOT_FOUND"
  suggestion?: string;
  context?: Record<string, any>;
}
```

**Edge Cases**:
- EC-013.1: FFI type conversion (number overflow, string encoding) → System shall handle gracefully
- EC-013.2: Null/undefined handling in TypeScript → System shall convert to appropriate Rust types
- EC-013.3: Async operation cancellation → System shall support cancellation tokens
- EC-013.4: Memory leak prevention → System shall properly cleanup FFI resources

**Unhappy Paths**:
- UP-013.1: FFI call fails (Rust panic) → System shall catch panic and return error
- UP-013.2: TypeScript receives invalid data from Rust → System shall validate and return error
- UP-013.3: FFI timeout (operation takes too long) → System shall support configurable timeout
- UP-013.4: FFI memory limit exceeded → System shall return error with memory stats

---

### REQ-014: CLI Application

**Description**: System shall provide CLI with REPL and one-shot commands.

**Acceptance Criteria**:
```
When user executes: zlf repl
Then:
  - Interactive REPL starts with prompt "zlf> "
  - Command history is maintained (up/down arrows)
  - Ctrl+C exits gracefully
  - Tab completion for commands

When user executes: zlf query "..."
Then:
  - Query executes and returns JSON result
  - Errors are formatted with error code and suggestion
```

**CLI Interface (JSON-over-STDIO)**:
```bash
# Initialize database
echo '{"command":"init","path":"./db"}' | zlf

# Add a node
echo '{"command":"add_node","path":"./db","labels":["person"],"properties":{"name":"Alice"}}' | zlf

# Get a node
echo '{"command":"get_node","path":"./db","id":"<node-id>"}' | zlf

# Add an edge
echo '{"command":"add_edge","path":"./db","edge_type":"knows","source":"<src>","target":"<tgt>","properties":{}}' | zlf

# Execute query
echo '{"command":"query","path":"./db","query":"node(person, X, _)."}' | zlf

# BM25 search
echo '{"command":"search","path":"./db","query":"engineer"}' | zlf

# Semantic search
echo '{"command":"similar","path":"./db","node_id":"<id>","threshold":0.8,"limit":10}' | zlf
```

**Edge Cases**:
- EC-014.1: Invalid JSON input → System shall return error with INVALID_REQUEST code
- EC-014.2: Missing required fields → System shall return error with missing field name
- EC-014.3: File permission issues → System shall return clear error message
- EC-014.4: Empty input → System shall skip and wait for next command
- EC-014.5: Unicode/emoji handling → System shall support UTF-8 in JSON

**Unhappy Paths**:
- UP-014.1: Invalid command type → System shall return error with available commands
- UP-014.2: Invalid JSON syntax → System shall return error with parse details
- UP-014.3: Missing required arguments → System shall return error with expected args
- UP-014.4: Database not found → System shall return DB_OPEN_FAILED error
- UP-014.5: Node/edge not found → System shall return NODE_NOT_FOUND or EDGE_NOT_FOUND error

## 11. Decisions Made

| Decision | Choice | Rationale |
|---|---|---|
| Query Language | Full Prolog syntax (zlf-log) | For AI agents, traditional syntax aids adoption |
| Project Name | zlf | Simple, memorable |
| Memory Lifecycle | Auto-expire + Importance | Balances retention with manageability |
| Schema Strategy | Schema-free | Flexibility for diverse knowledge sources |
| Inference Level | Rule inference | Sufficient for knowledge discovery |
| Data Exchange | JSON + Auto-parse Docs | Covers structured + unstructured sources |
| Storage Backend | Embedded KV Store (RocksDB) | Performance, embedded, no external deps |
| Embedding Model | Pluggable | User choice, flexibility |
| Temporal Granularity | Node Versioning | Nodes are primary entities, edges reference versions |
| Import Pipeline | LLM + Human Review | Accuracy + control |
| Implementation | Rust (engine) + TypeScript (SDK) | Performance + ecosystem |
| Query Engine | WAM in Rust | Efficient Prolog execution |
| File Format | Directory-based | Separation of concerns, extensibility |
| Query Result | JSON | AI-friendly |
| Error Handling | Detailed Errors | Developer experience |
| Concurrency | Read-Write Lock | Read-heavy workload |
| Prolog-Graph Integration | Dynamic Facts | node/edge as Prolog facts, generated on query |
| Memory Structure | Typed Nodes | Each memory is a node with type (conversation/knowledge/task) |
| CLI Model | JSON-over-STDIO | Simple, scriptable, no REPL needed |
| Embedding Integration | Per-node Embedding | Optional embedding per node for semantic search |
| Error Recovery | WAL Recovery | Crash recovery to consistent state |
| Testing Strategy | Hybrid | Prolog tests for business logic + SDK tests for integration |
| v1 Integration | CLI + SDK Only | HTTP service deferred to v2 |
| Versioning Strategy | Optimistic Locking | Detect conflicts and let user decide |
| Backup/Restore | Directory Snapshot | Simple, reliable, easy to implement |
| Monitoring | All Metrics | Graph stats, query performance, resource usage, write ops |
| Documentation | Markdown in Repo | Version controlled, easy to maintain |
| Conflict Resolution | Auto-merge + Error | Auto-merge simple conflicts, error with details for complex |
| Max Scale (v1) | 100K Nodes, 1M Edges | Sufficient for team-scale use cases |
| Large Properties | Overflow Storage | Large values stored separately, referenced by pointer |
| Overflow Implementation | KV Reference | Large properties stored as separate KV entries |
| Schema Migration | No Migration Needed | Schema-free design eliminates migration |
| Embedding Versioning | Metadata per Embedding | Each embedding stores model name and version |
| KV Schema | Simple KV | key=uuid, value=serialized node/edge |
| WAL Implementation | RocksDB WAL | Use RocksDB's built-in WAL |
| WAM Testing | Layered Testing | Instruction-level unit tests + integration tests |
| Serialization | Bincode | Binary format, efficient and compact |
| Prolog Parser | Parser Generator | Use pest or lalrpop for parser generation |
| FFI Strategy | JSON-over-STDIO | Rust CLI binary + TypeScript child_process, simple + portable |
| Project Structure | Monorepo | Rust crate + TypeScript package in one repo |
| CI/CD | GitHub Actions | Automated testing and release |
| Version Numbering | SemVer | Semantic versioning (major.minor.patch) |
| Project Skeleton | Full Structure | Complete structure with all modules |
| Semantic Search | Embedding as Property | Embedding stored as node property, similarity computed at query time |
| BM25 Support | BM25 in v1 | Inverted index on text properties for keyword search |
| Temporal Engine | Time Index | Maintain time index for efficient time range queries |
| Mixed Query Order | Graph → Semantic → Time | Graph traversal first, then semantic filtering, then time filtering |
| BM25 Integration | Inverted Index | Build inverted index on node text properties |
| Temporal Syntax | Built-in Time Functions | time_range, before, after, between functions |
| Semantic Query API | similar_to Function | similar_to(Node, Threshold, Results) for similarity search |
| Query Combination | Set-based | All queries return node sets, can be combined |
| Storage Crate | rust-rocksdb | Official RocksDB binding, mature and stable |
| Parser Crate | pest | PEG parser generator, flexible and well-documented |
| CLI Crate | serde_json + anyhow | JSON serialization + error handling for STDIO |
| Serialization | serde + bincode | Standard serialization, efficient binary format |
| Error Handling | thiserror + anyhow | Standard error handling pattern |
| Logging | tracing | Modern async logging framework |
| FFI | JSON-over-STDIO | Rust CLI binary + TypeScript child_process |
| Search | tantivy | Open-source full-text search engine (BM25) |
| Chinese Segmentation | jieba-rs | Chinese word segmentation for BM25 |

## 12. Open Questions

- [ ] None - ready for solution design
