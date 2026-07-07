# Change Note 005: Configurable Embedding Provider

## Linked Work

- Requirements / track: docs/track/zlf/prd-v1.md (REQ-007)
- Solution design: docs/track/zlf/solution-design-v1.md
- Plan: docs/track/zlf/plan-v1.md (Slice 11)
- Delivery record: docs/track/zlf/delivery-record-v1.md

## Discovery Phase

build

## Original Decision

Vector index existed but had no embedding provider. Users had to manually provide embeddings.

## Problem Found

1. No way to generate embeddings from text
2. No integration with embedding APIs (OpenAI, Ollama, HuggingFace)
3. Semantic search required pre-computed embeddings

## New Decision

### 1. Embedding Provider Crate (`zlf-embed`)
- Created new crate for embedding generation
- Supports multiple providers: Ollama, OpenAI, HuggingFace
- Configurable: API endpoint, API key, model ID, dimension
- Async interface with `async-trait`

### 2. CLI Integration
- Added `embed` command: Generate embedding for text
- Added `index_embedding` command: Generate and store embedding for node
- Configuration via JSON request

### 3. QueryPlanner Integration
- Added `index_embedding` method to store embeddings
- Vector search now works with generated embeddings

## Impact

- User behavior: Can now generate embeddings and use semantic search
- Modules/files:
  - Created: `crates/zlf-embed/` (new crate)
  - Modified: `crates/zlf-cli/src/main.rs` (embed commands)
  - Modified: `crates/zlf-query/src/lib.rs` (index_embedding method)
  - Modified: `crates/zlf-index/src/lib.rs` (export VectorEntry)
- Data/contracts: New embed/index_embedding commands
- Tests/verification: Embedding provider tests passing
- Cross-feature knowledge to update in `docs/knowledge`: Embedding integration complete
- Risks: Requires external API (Ollama/OpenAI) for embedding generation

## Approval / Rationale

Autonomous change - completes embedding functionality per PRD REQ-007.

## Verification Update

- `cargo build -p zlf-embed` → Success
- `cargo build -p zlf-cli` → Success
- Ollama embedding test → Success (1024-dim vector generated)

## Scope Reduction

- Original scope items removed: None
- Reason: N/A
- Impact on later phases: None
- Deferred decisions: None
- Revisit trigger: None
