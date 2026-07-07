# Change Note 004: Temporal Index Integration + Embedding Provider Design

## Linked Work

- Requirements / track: docs/track/zlf/prd-v1.md (REQ-005, REQ-007)
- Solution design: docs/track/zlf/solution-design-v1.md
- Plan: docs/track/zlf/plan-v1.md (Slice 10, 11)
- Delivery record: docs/track/zlf/delivery-record-v1.md

## Discovery Phase

build

## Original Decision

Temporal index existed but was unused. Vector index had no embedding provider. Query planner had stub implementations for time_range, before, after.

## Problem Found

1. `query_time_range`, `query_before`, `query_after` returned empty vectors
2. No embedding provider - users had to manually provide vectors
3. No temporal indexing when nodes are created/updated
4. System was a "toy" with working core but missing advanced features

## New Decision

### 1. Temporal Index Integration
- Implemented `query_time_range(node_id, start, end)` - query nodes in time range
- Implemented `query_before(node_id, time)` - query nodes before time
- Implemented `query_after(node_id, time)` - query nodes after time
- Added automatic temporal indexing when nodes are created
- Added `index_temporal` CLI command for manual indexing

### 2. Embedding Provider Design (Pending)
- Configurable provider: Ollama, OpenAI-compatible, HuggingFace
- Configuration: API endpoint, API key, model ID
- Auto-embedding on node creation (optional)

## Impact

- User behavior: Temporal queries now work
- Modules/files:
  - Modified: `crates/zlf-query/src/lib.rs` (temporal query methods)
  - Modified: `crates/zlf-cli/src/main.rs` (index_temporal command)
- Data/contracts: New temporal query syntax
- Tests/verification: 8 query tests passing
- Cross-feature knowledge to update in `docs/knowledge`: Temporal integration complete
- Risks: None

## Approval / Rationale

Autonomous change - completes temporal index functionality per PRD REQ-005.

## Verification Update

- `cargo test -p zlf-query` → 8 tests passing
- Temporal queries: `?time_range(node_id, start, end).` working
- Before/after queries: `?before(node_id, time).` working

## Scope Reduction

- Original scope items removed: None
- Reason: N/A
- Impact on later phases: None
- Deferred decisions: Embedding provider implementation
- Revisit trigger: None
