# Plan v1: zlf - AI-Native Graph Database with Logic Reasoning

## Goal

Implement zlf v1 with core graph storage, zlf-log query language, CLI, and TypeScript SDK for AI agent memory management.

## Source Artifacts

- PRD / track: docs/track/zlf/prd-v1.md
- Solution design: docs/track/zlf/solution-design-v1.md
- Prior delivery records: None (new project)
- Change notes: None

## Constraints and Non-Goals

- **Max Scale**: 100K nodes, 1M edges
- **No distributed mode**: Single-node only
- **No graph algorithms**: Focus on Prolog inference
- **No visualization**: CLI + SDK only
- **TypeScript only as wrapper**: No Python SDK in v1
- **Prolog engine in Rust**: Parser, WAM, and all inference logic in Rust

## Task Slices

### Slice 1: Project Setup & Core Types

**Acceptance covered:** REQ-001, REQ-002

**Files / modules:**
- Create: Cargo.toml (workspace), crates/zlf-core/
- Modify: None
- Test: crates/zlf-core/src/node.rs, edge.rs

**Verification:**
- `cargo build` → success
- `cargo test -p zlf-core` → all tests pass

**Edge Cases Covered:**
- EC-001.1: Empty labels array
- EC-001.2: Empty properties object
- EC-001.3: Nested properties
- EC-001.4: Large properties (>1KB)
- EC-002.1: Edge with no properties
- EC-002.2: Self-referencing edge
- EC-002.3: Multiple edges between same nodes

**Unhappy Paths Covered:**
- UP-001.1: Duplicate node ID
- UP-001.2: Invalid property value type
- UP-001.3: Node ID exceeds max length
- UP-002.1: Source node does not exist
- UP-002.2: Target node does not exist
- UP-002.3: Edge type is empty

**Steps:**
- [x] Create Rust workspace with cargo
- [x] Implement Node struct with labels and properties
- [x] Implement Edge struct with type, source, target
- [x] Implement Value enum for properties (String, Number, Bool, Null, Array, Object)
- [x] Add serialization with Bincode
- [x] Write unit tests for Node and Edge (including edge cases)

---

### Slice 2: Storage Engine

**Acceptance covered:** REQ-001, REQ-002, REQ-009

**Files / modules:**
- Create: crates/zlf-storage/
- Modify: None
- Test: crates/zlf-storage/src/kv.rs

**Verification:**
- `cargo test -p zlf-storage` → all tests pass
- Manual: Create database, add node, retrieve node

**Edge Cases Covered:**
- EC-009.1: Update with same properties (still creates version)
- EC-009.2: Concurrent updates (optimistic locking)

**Unhappy Paths Covered:**
- UP-009.1: Version conflict
- UP-009.2: Max versions exceeded

**Steps:**
- [x] Initialize RocksDB dependency
- [x] Implement KV store wrapper
- [x] Implement node storage (create, read, update, delete)
- [x] Implement edge storage (create, read, update, delete)
- [x] Implement WAL configuration
- [x] Implement version management
- [x] Write integration tests (including edge cases)

---

### Slice 3: Index Management

**Acceptance covered:** REQ-005, REQ-007, REQ-008

**Files / modules:**
- Create: crates/zlf-index/
- Modify: None
- Test: crates/zlf-index/src/temporal.rs, bm25.rs, vector.rs

**Verification:**
- `cargo test -p zlf-index` → all tests pass
- Manual: Query by time, search by text, find similar nodes

**Edge Cases Covered:**
- EC-005.1: Node with single version
- EC-005.2: Node with overlapping versions
- EC-007.1: No nodes with embeddings
- EC-007.2: Node with multiple embedding versions
- EC-008.1: Empty search query
- EC-008.2: Search with special characters

**Unhappy Paths Covered:**
- UP-005.1: Time index corrupted
- UP-007.1: Embedding dimension mismatch
- UP-007.2: Embedding provider unavailable
- UP-008.1: BM25 index not built
- UP-008.2: Search timeout

**Steps:**
- [x] Implement label index (label -> node IDs)
- [x] Implement edge type index (type -> edge IDs)
- [x] Implement temporal index (timestamp -> node IDs)
- [x] Implement BM25 inverted index
- [x] Implement vector index interface (pluggable)
- [x] Write unit tests for each index type (including edge cases)

---

### Slice 4: Prolog Parser

**Acceptance covered:** REQ-003

**Files / modules:**
- Create: crates/zlf-prolog/src/lexer.rs, parser.rs, ast.rs
- Modify: None
- Test: crates/zlf-prolog/tests/parser_test.rs

**Verification:**
- `cargo test -p zlf-prolog --test parser_test` → all tests pass
- Manual: Parse zlf-log queries

**Edge Cases Covered:**
- EC-003.1: Rule with recursive definition
- EC-003.2: Rule with multiple clauses
- EC-003.3: Rule with built-in predicates

**Unhappy Paths Covered:**
- UP-003.1: Invalid Prolog syntax
- UP-003.2: Undefined predicate in rule
- UP-003.3: Rule exceeds max complexity

**Steps:**
- [x] Define grammar (pest or lalrpop)
- [x] Implement lexer (tokenize zlf-log)
- [x] Implement parser for facts, rules, queries
- [x] Implement AST types
- [x] Write parser tests for valid/invalid syntax (including edge cases)

---

### Slice 5: WAM Execution Engine

**Acceptance covered:** REQ-004

**Files / modules:**
- Create: crates/zlf-prolog/src/wam.rs, unification.rs, builtin.rs
- Modify: None
- Test: crates/zlf-prolog/tests/wam_test.rs

**Verification:**
- `cargo test -p zlf-prolog --test wam_test` → all tests pass
- Manual: Execute simple Prolog queries

**Edge Cases Covered:**
- EC-004.1: Query with no matching results
- EC-004.2: Query with infinite recursion (depth limit)
- EC-004.3: Query combining graph, semantic, and temporal
- EC-004.4: Query with multiple rules
- EC-004.5: Query with variable binding across clauses
- EC-004.6: Query with negation (\=)

**Unhappy Paths Covered:**
- UP-004.1: Query references undefined predicate
- UP-004.2: Query exceeds max execution time (30s)
- UP-004.3: Query exceeds max result count (10000)
- UP-004.4: Query has syntax error
- UP-004.5: Query uses unsupported feature

**Steps:**
- [x] Implement WAM instructions (put, get, unify, etc.)
- [x] Implement choice points and backtracking
- [x] Implement unification algorithm
- [x] Implement built-in predicates (time, search, similar)
- [x] Write WAM instruction tests (including edge cases)

---

### Slice 6: Query Planner & Executor

**Acceptance covered:** REQ-004, REQ-005, REQ-007, REQ-008

**Files / modules:**
- Create: crates/zlf-query/src/planner.rs, executor.rs, optimizer.rs
- Modify: None
- Test: crates/zlf-query/tests/integration_test.rs

**Verification:**
- `cargo test -p zlf-query --test integration_test` → all tests pass
- Manual: Execute complex queries with semantic/temporal filters

**Edge Cases Covered (from multiple requirements, relevant to query planning):**
- EC-004.3: Query combining graph, semantic, and temporal (execution order) - from REQ-004
- EC-004.4: Query with multiple rules (planner merges results) - from REQ-004
- EC-004.5: Query with variable binding across clauses (planner tracks bindings) - from REQ-004
- EC-006.1: Memory query with no entities (planner skips entity filter) - from REQ-006
- EC-007.3: Semantic search with threshold = 0.0 (planner returns all) - from REQ-007
- EC-008.3: BM25 search across multiple property types (planner aggregates) - from REQ-008

**Unhappy Paths Covered (from multiple requirements, relevant to query planning):**
- UP-004.1: Query references undefined predicate (planner validation) - from REQ-004
- UP-004.4: Query has syntax error (planner returns parse error) - from REQ-004
- UP-004.5: Query uses unsupported feature (planner returns feature error) - from REQ-004

**Steps:**
- [x] Implement query planner (graph -> semantic -> time)
- [x] Implement query executor
- [x] Integrate with zlf-prolog for parsing and execution
- [x] Implement result serialization to JSON
- [x] Write integration tests (including edge cases)

---

### Slice 7: Node Versioning & Temporal

**Acceptance covered:** REQ-005, REQ-006, REQ-009

**Files / modules:**
- Create: crates/zlf-storage/src/version.rs
- Modify: crates/zlf-storage/src/kv.rs
- Test: crates/zlf-storage/tests/version_test.rs

**Verification:**
- `cargo test -p zlf-storage --test version_test` → all tests pass
- Manual: Query node at specific time

**Edge Cases Covered:**
- EC-005.1: Node with single version
- EC-005.2: Node with overlapping versions
- EC-006.1: Memory with no entities
- EC-006.2: Memory with importance > 1.0
- EC-006.3: Memory with TTL = 0
- EC-009.1: Update with same properties

**Unhappy Paths Covered:**
- UP-005.1: Time index corrupted
- UP-006.1: Storage full
- UP-006.2: Invalid memory type
- UP-009.1: Version conflict
- UP-009.2: Max versions exceeded

**Steps:**
- [ ] Implement node version storage
- [ ] Implement version creation on update
- [ ] Implement temporal queries (time_range, before, after)
- [ ] Implement time index integration
- [ ] Write temporal query tests (including edge cases)

---

### Slice 8: TypeScript SDK (JSON-over-STDIO)

**Acceptance covered:** REQ-013, All requirements (via JSON-over-STDIO)

**Files / modules:**
- Create: packages/zlf/, crates/zlf-api/, crates/zlf-cli/
- Modify: None
- Test: packages/zlf/src/__tests__/

**Verification:**
- `npm test` → all tests pass
- `cargo test -p zlf-cli` → all tests pass
- Manual: Use SDK to create nodes and query

**Edge Cases Covered:**
- EC-013.1: JSON type conversion (number overflow, string encoding) - from REQ-013
- EC-013.2: Null/undefined handling in TypeScript - from REQ-013
- EC-013.3: Async operation timeout - from REQ-013
- EC-013.4: Process cleanup - from REQ-013

**Unhappy Paths Covered:**
- UP-013.1: Rust CLI process fails - from REQ-013
- UP-013.2: TypeScript receives invalid JSON from Rust - from REQ-013
- UP-013.3: CLI timeout - from REQ-013
- UP-013.4: Binary not found - from REQ-013

**Steps:**
- [x] Implement Rust CLI binary (JSON-over-STDIO)
- [x] Implement TypeScript ZLF class (calls Rust CLI)
- [x] Implement types.ts (TypeScript types)
- [x] Write unit tests (mocked)
- [x] Write integration tests (real binary)

---

### Slice 9: CLI Application (Rust Binary)

**Acceptance covered:** REQ-014, All requirements (via Rust CLI)

**Files / modules:**
- Create: crates/zlf-cli/
- Modify: None
- Test: crates/zlf-cli/tests/

**Verification:**
- `cargo test -p zlf-cli` → all tests pass
- Manual: Use CLI commands via STDIN/STDOUT

**Edge Cases Covered:**
- EC-014.1: Invalid JSON input - from REQ-014
- EC-014.2: Missing required fields - from REQ-014
- EC-014.3: File permission issues - from REQ-014
- EC-014.4: Empty input handling - from REQ-014
- EC-014.5: Unicode/emoji handling - from REQ-014

**Unhappy Paths Covered:**
- UP-014.1: Invalid command type - from REQ-014
- UP-014.2: Invalid JSON syntax - from REQ-014
- UP-014.3: Missing required arguments - from REQ-014
- UP-014.4: Database not found - from REQ-014
- UP-014.5: Node/edge not found - from REQ-014

**Steps:**
- [x] Implement JSON request/response types
- [x] Implement command handlers
- [x] Implement STDIO loop
- [x] Write integration tests
- [x] Add to workspace

---

### Slice 10: BM25 Search Integration

**Acceptance covered:** REQ-008

**Files / modules:**
- Create: crates/zlf-index/src/bm25.rs
- Modify: crates/zlf-query/src/executor.rs
- Test: crates/zlf-index/tests/bm25_test.rs

**Verification:**
- `cargo test -p zlf-index --test bm25_test` → all tests pass
- Manual: Search by text query

**Edge Cases Covered:**
- EC-008.1: Empty search query
- EC-008.2: Search with special characters
- EC-008.3: Search across multiple property types
- EC-008.4: Mixed Chinese and English text
- EC-008.5: Chinese punctuation

**Unhappy Paths Covered:**
- UP-008.1: BM25 index not built
- UP-008.2: Search timeout
- UP-008.3: Jieba segmentation failure

**Steps:**
- [ ] Implement BM25 scoring algorithm
- [ ] Implement inverted index
- [ ] Implement search function
- [ ] Integrate with query executor
- [ ] Write BM25 tests (including edge cases)

---

### Slice 11: Semantic Search Integration

**Acceptance covered:** REQ-007

**Files / modules:**
- Create: crates/zlf-index/src/vector.rs
- Modify: crates/zlf-query/src/executor.rs
- Test: crates/zlf-index/tests/vector_test.rs

**Verification:**
- `cargo test -p zlf-index --test vector_test` → all tests pass
- Manual: Find similar nodes

**Edge Cases Covered:**
- EC-007.1: No nodes with embeddings
- EC-007.2: Node with multiple embedding versions
- EC-007.3: Threshold = 0.0
- EC-007.4: Threshold = 1.0
- EC-007.5: Query node has no embedding
- EC-007.6: Different embedding models

**Unhappy Paths Covered:**
- UP-007.1: Embedding dimension mismatch
- UP-007.2: Embedding provider unavailable
- UP-007.3: Embedding API rate limit
- UP-007.4: Embedding generation timeout
- UP-007.5: Invalid embedding values

**Steps:**
- [ ] Implement vector index interface
- [ ] Implement pluggable embedding provider
- [ ] Implement similarity search
- [ ] Integrate with query executor
- [ ] Write vector search tests (including edge cases)

---

### Slice 12: Import/Export & Documentation

**Acceptance covered:** REQ-010, REQ-011, REQ-012

**Files / modules:**
- Create: cli/src/commands/import.ts, export.ts
- Modify: None
- Test: cli/tests/import_export_test.ts

**Verification:**
- `npm test` → all tests pass
- Manual: Import/export JSON data

**Edge Cases Covered:**
- EC-010.1: Import with duplicate IDs
- EC-010.2: Import with invalid references
- EC-010.3: Export empty database
- EC-011.1: Init existing database
- EC-011.2: Backup to existing file
- EC-011.3: Restore to non-empty database
- EC-012.1: Error in nested operation
- EC-012.2: Error during transaction
- EC-012.3: Multiple errors in batch operation
- EC-012.4: Error in async operation

**Unhappy Paths Covered:**
- UP-010.1: Invalid JSON format
- UP-010.2: File not found
- UP-010.3: Permission denied
- UP-011.1: Disk full
- UP-011.2: Corrupted backup
- UP-012.1: Error logging fails
- UP-012.2: Error causes panic
- UP-012.3: Error response too large
- UP-012.4: Error in error handler

**Steps:**
- [ ] Implement JSON import
- [ ] Implement JSON export
- [ ] Implement document auto-parse (basic)
- [ ] Write documentation
- [ ] Write import/export tests (including edge cases)

## Acceptance Mapping

| Acceptance / Req | Slice | Edge Cases | Unhappy Paths | Verification |
|---|---|---|---|---|
| REQ-001: Node storage | Slice 1, 2 | EC-001.1, EC-001.2, EC-001.3, EC-001.4 | UP-001.1, UP-001.2, UP-001.3 | Unit tests, integration tests |
| REQ-002: Edge storage | Slice 1, 2 | EC-002.1, EC-002.2, EC-002.3 | UP-002.1, UP-002.2, UP-002.3 | Unit tests, integration tests |
| REQ-003: Rule compilation | Slice 4 | EC-003.1, EC-003.2, EC-003.3 | UP-003.1, UP-003.2, UP-003.3 | Parser tests |
| REQ-004: Backtracking | Slice 5, 6 | EC-004.1, EC-004.2, EC-004.3, EC-004.4, EC-004.5, EC-004.6 | UP-004.1, UP-004.2, UP-004.3, UP-004.4, UP-004.5 | WAM tests, integration tests |
| REQ-005: Temporal filters | Slice 3, 7 | EC-005.1, EC-005.2, EC-005.3 | UP-005.1, UP-005.2 | Temporal query tests |
| REQ-006: Temporal memory | Slice 7 | EC-006.1, EC-006.2, EC-006.3 | UP-006.1, UP-006.2 | Version tests |
| REQ-007: Semantic search | Slice 3, 11 | EC-007.1, EC-007.2, EC-007.3, EC-007.4, EC-007.5, EC-007.6 | UP-007.1, UP-007.2, UP-007.3, UP-007.4, UP-007.5 | Vector search tests |
| REQ-008: BM25 search | Slice 3, 10 | EC-008.1, EC-008.2, EC-008.3, EC-008.4, EC-008.5 | UP-008.1, UP-008.2, UP-008.3 | BM25 tests |
| REQ-009: Node versioning | Slice 2, 7 | EC-009.1, EC-009.2 | UP-009.1, UP-009.2 | Version tests |
| REQ-010: Import/Export | Slice 12 | EC-010.1, EC-010.2, EC-010.3 | UP-010.1, UP-010.2, UP-010.3 | Import/export tests |
| REQ-011: Database management | Slice 12 | EC-011.1, EC-011.2, EC-011.3 | UP-011.1, UP-011.2 | CLI tests |
| REQ-012: Error handling | Slice 12 | EC-012.1, EC-012.2, EC-012.3, EC-012.4 | UP-012.1, UP-012.2, UP-012.3, UP-012.4 | Integration tests |
| REQ-013: TypeScript SDK | Slice 8 | EC-013.1, EC-013.2, EC-013.3, EC-013.4 | UP-013.1, UP-013.2, UP-013.3, UP-013.4 | SDK tests |
| REQ-014: CLI Application | Slice 9 | EC-014.1, EC-014.2, EC-014.3, EC-014.4, EC-014.5 | UP-014.1, UP-014.2, UP-014.3, UP-014.4, UP-014.5 | CLI tests |

## Stop Conditions

Pause and revise if:

- Unplanned module/package changes are required
- Contract changes (API, data format)
- Touched files/modules exceed estimate by ~2x
- Acceptance becomes invalid or untestable
- Performance issues with 100K nodes
- RocksDB integration issues

## Risks / Rollback

| Risk | Mitigation | Rollback |
|---|---|---|
| RocksDB complexity | Use well-tested Rust bindings | Fall back to SQLite |
| WAM implementation bugs | Comprehensive test suite | Use interpreter instead of compiled |
| FFI overhead | Optimize data serialization | Use IPC instead of FFI |
| Performance issues | Profile early, optimize hot paths | Reduce scope (no semantic search) |
| Parser complexity | Start with subset, extend gradually | Use existing Prolog parser |
| Prolog syntax complexity | Implement core features first, extend incrementally | Simplify to basic pattern matching |
| FFI compatibility | Test on multiple Node.js versions, pin napi-rs version | Use child process instead of FFI |
