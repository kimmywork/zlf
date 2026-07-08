# zlf Implementation Checklist

## 1. Core Storage

| Feature | PRD | Status | Notes |
|---------|-----|--------|-------|
| Node CRUD | REQ-001 | ✅ | |
| Edge CRUD | REQ-002 | ✅ | |
| Node versioning | REQ-009 | ✅ | |
| Temporal queries | REQ-005 | ✅ | |
| Node ID validation | UP-001.3 | ✅ | Max 255 chars |
| Duplicate node detection | UP-001.1 | ✅ | |
| Empty labels/properties | EC-001.1/2 | ✅ | |
| Nested properties | EC-001.3 | ✅ | |
| Self-referencing edges | EC-002.2 | ✅ | |

## 2. Query Engine

| Feature | PRD | Status | Notes |
|---------|-----|--------|-------|
| Basic node query | REQ-004 | ✅ | `?node(label, X, Props).` |
| Basic edge query | REQ-004 | ✅ | `?edge(type, X, Y, Props).` |
| Query by label | REQ-004 | ✅ | |
| Query by edge type | REQ-004 | ✅ | |
| Query all nodes | REQ-004 | ✅ | `?node(X, Y, Z).` |
| **Rule definition** | REQ-003 | ⚠️ | Rules stored in memory, not persisted |
| **Rule execution** | REQ-004 | ⚠️ | Simplified - no full backtracking |
| **Backtracking** | REQ-004 | ❌ | Not fully implemented |
| **Recursive rules** | EC-003.1 | ❌ | Not implemented |
| **Built-in predicates** | EC-003.3 | ❌ | Not implemented |
| Invalid syntax handling | UP-003.1 | ✅ | Returns error |
| Query timeout | UP-004.2 | ❌ | Not implemented |

## 3. BM25 Search

| Feature | PRD | Status | Notes |
|---------|-----|--------|-------|
| BM25 index | REQ-008 | ✅ | |
| Chinese tokenization | REQ-008 | ✅ | jieba-rs |
| Search function | REQ-008 | ✅ | |
| Empty query handling | EC-008.1 | ✅ | Returns error |
| Special characters | EC-008.2 | ✅ | |
| Mixed language | EC-008.4 | ✅ | |
| **Auto-indexing** | - | ✅ | Text properties indexed on creation |

## 4. Semantic Search

| Feature | PRD | Status | Notes |
|---------|-----|--------|-------|
| Vector index | REQ-007 | ✅ | |
| Similarity search | REQ-007 | ✅ | |
| Cosine similarity | REQ-007 | ✅ | |
| Threshold filtering | EC-007.3/4 | ✅ | |
| Dimension mismatch | UP-007.1 | ✅ | Skips mismatched |
| No embedding handling | EC-007.5 | ✅ | Returns error |
| **Auto-embedding** | - | ❌ | Must call index_embedding manually |
| **Embedding provider** | REQ-007 | ✅ | Ollama/OpenAI |

## 5. Temporal

| Feature | PRD | Status | Notes |
|---------|-----|--------|-------|
| Node versioning | REQ-009 | ✅ | |
| Temporal index | REQ-005 | ✅ | |
| time_range query | REQ-005 | ✅ | |
| before query | REQ-005 | ✅ | |
| after query | REQ-005 | ✅ | |
| Version creation on update | EC-009.1 | ✅ | |

## 6. Import/Export

| Feature | PRD | Status | Notes |
|---------|-----|--------|-------|
| JSON import | REQ-010 | ✅ | |
| JSON export | REQ-010 | ✅ | |
| Duplicate handling | EC-010.1 | ⚠️ | Skips with warning |
| Invalid references | EC-010.2 | ⚠️ | Skips with warning |

## 7. CLI Interface

| Feature | PRD | Status | Notes |
|---------|-----|--------|-------|
| JSON-over-STDIO | REQ-014 | ✅ | |
| HTTP server mode | - | ✅ | axum |
| Connection pooling | - | ✅ | RwLock |
| Health endpoint | - | ✅ | /health |
| Error responses | REQ-012 | ✅ | Structured JSON |

## 8. Configuration

| Feature | Status | Notes |
|---------|--------|-------|
| Config file | ✅ | zlf.json |
| Default db_path | ✅ | |
| Embedding config | ✅ | |
| Config get/set | ✅ | CLI command |

## Missing Features Summary

### Critical (must implement)
1. **Rule storage and execution** - Core Prolog functionality
2. **Backtracking** - Required for multi-result queries
3. **Auto-indexing** - Should index on node creation

### Important (should implement)
4. **Query timeout** - Prevent infinite loops
5. **Built-in predicates** - time, search, similar in queries
6. **Recursive rules** - Variable-length paths

### Nice to have
7. **Batch operations** - Bulk insert/update
8. **Connection pooling tuning** - Optimize pool size
9. **Query plan optimization** - Optimize execution order
