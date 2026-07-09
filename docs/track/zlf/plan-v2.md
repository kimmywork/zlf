---
status: in_progress
owner: kimmy
updated: 2026-07-08
scope_type: parent
source_requirements:
  - docs/track/zlf/requirements-v1.md
  - docs/track/zlf/change-note-006.md
---

# Plan v2: WAM-backed Prolog + DB Fact Space + Knowledge Import

## Goal

Realign zlf with the original target:

> Use a real WAM-style Prolog runtime whose fact space is backed directly by RocksDB graph storage and database indexes, while supporting graph/property/BM25/vector/temporal composition, persistent indexed rules, Markdown knowledge-base import, JSON-over-stdio, and HTTP SSE integration.

## Current State Summary

- Main execution path currently uses `PrologEngine`, which is an AST interpreter / Prolog-like executor, not a true WAM runtime.
- Legacy WAM prototypes (`wam.rs`, `wam_enhanced.rs`, `wam_v2.rs`) were unused by the main path and have been removed.
- Database-backed graph/property/search/vector/temporal predicate resolution exists, but is currently embedded in the interpreter rather than a clean WAM `FactProvider` interface.
- Rule persistence by predicate key exists.
- Markdown import exists at document/chunk/contains-edge level.
- SSE endpoint exists, with query chunk events as the initial streaming foundation.
- rustfmt/clippy/source-size gates have been added, but strict `too_many_lines=30` and source file-size limits are not fully green yet.

## Workstream A: Quality Gate and Source Layout

### A1. Enforce Rust quality gates

**Status:** in_progress

**Target checks:**

```bash
cargo fmt --all -- --check
python3 scripts/check-rust-size.py
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
```

**Limits:**

- `too-many-lines-threshold = 30`
- normal Rust source file max: 300 lines
- test Rust file max: 400 lines

**Remaining:**

- Split files still over max lines.
- Refactor functions still over 30 lines.
- Avoid broad `allow` exceptions except for explicitly documented temporary migration points.

### A2. Split oversized files

**Status:** todo

| File | Current issue | Target split |
|---|---|---|
| `crates/zlf-cli/src/main.rs` | >300 lines | `request.rs`, `stdio.rs`, `http.rs`, `sse.rs`, `import.rs`, `export.rs`, `config.rs`, `main.rs` |
| `crates/zlf-cli/tests/integration_test.rs` | >400 lines | split by command group |
| `crates/zlf-embed/src/lib.rs` | >300 lines | provider modules |
| `crates/zlf-index/src/temporal.rs` | >300 lines incl. tests | move tests / split helpers |
| `crates/zlf-prolog/src/parser.rs` | >300 lines, long fns | `ast.rs`, `parser.rs`, `convert.rs`, parser tests |
| `crates/zlf-prolog/src/prolog_engine.rs` | >300 lines, long fns | temporary `interpreter/` modules, then WAM replacement |
| `crates/zlf-query/src/lib.rs` | >300 lines | `planner.rs`, `rules.rs`, `indexing.rs`, `execute.rs` |
| `crates/zlf-storage/src/lib.rs` | >300 lines | `node.rs`, `edge.rs`, `version.rs`, `rules.rs`, `raw.rs`, `memory.rs` |

## Workstream B: Real WAM Runtime

### B1. Rename current engine role

**Status:** todo

- Rename or logically separate current `PrologEngine` as `AstInterpreter` / `InterpreterEngine`.
- Keep it only as a transitional implementation while WAM runtime is built.
- Ensure public API does not falsely imply WAM semantics.

### B2. Define WAM architecture

**Status:** todo

New module target:

```text
crates/zlf-prolog/src/wam/
  mod.rs
  instruction.rs
  compiler.rs
  runtime.rs
  frame.rs
  choice_point.rs
  trail.rs
  heap.rs
  register.rs
  fact_provider.rs
```

### B3. WAM compiler

**Status:** todo

Compile parsed `Fact` / `PrologRule` / query terms into WAM-style instructions.

Minimum supported instruction groups:

- put/get variable
- put/get value
- put/get constant
- unify variable/value/constant
- call
- proceed
- try/retry/trust
- cut placeholder or explicit unsupported error

### B4. WAM runtime

**Status:** todo

Runtime requirements:

- X registers / environment frames
- heap term representation
- trail and variable unbinding
- choice points and backtracking
- recursion depth limit
- result limit
- structured errors

### B5. Compatibility tests

**Status:** todo

WAM must pass equivalent tests for:

- facts
- rules
- conjunction
- backtracking
- recursive rules
- DB-backed dynamic facts
- property predicates
- BM25/vector/temporal predicates

## Workstream C: DB-backed FactProvider

### C1. Define `FactProvider` trait

**Status:** todo

Core contract:

```rust
trait FactProvider {
    fn resolve(
        &self,
        predicate: PredicateKey,
        args: &[Term],
        bindings: &Bindings,
    ) -> Result<FactStream>;
}
```

### C2. RocksDB graph provider

**Status:** todo

Predicate mapping:

| Prolog predicate | DB source |
|---|---|
| `node(Label, Id, Props)` | node storage + label index |
| `edge(Type, Source, Target, Props)` | edge storage + edge_type index |
| `{edge_type}(Source, Target)` | edge_type index dynamic predicate |
| `{edge_type}(Source, Target, Props)` | edge_type index dynamic predicate |
| `prop(Entity, Key, Value)` | node/edge properties |
| `node_property/3` | node properties |
| `edge_property/3` | edge properties |

### C3. Index fact providers

**Status:** todo

| Predicate | Index |
|---|---|
| `search(Query, Node)` | BM25 |
| `search(Query, Node, Score)` | BM25 |
| `similar_to(Source, Threshold, Node)` | vector |
| `similar_to(Source, Threshold, Node, Score)` | vector |
| `after(Node, Date)` | temporal |
| `before(Node, Date)` | temporal |
| `time_range(Node, Start, End)` | temporal |

### C4. Binding-aware index selection

**Status:** todo

Use current bindings to avoid scans:

- bound label -> label index
- bound edge type -> edge index
- bound source/target -> future source/target index
- bound property key -> future property index
- bound search query -> BM25 index
- bound vector source -> vector index
- bound temporal range -> temporal index

## Workstream D: Persistent Indexed Rules

### D1. Rule storage

**Status:** done_initial

Current key format:

```text
rule:{predicate}:{uuid}
```

### D2. Rule index metadata

**Status:** todo

Add metadata keys:

```text
idx:rule_predicate:{predicate}:{rule_id}
idx:rule_arity:{predicate}:{arity}:{rule_id}
idx:rule_dependency:{dependency_predicate}:{rule_id}
```

### D3. Rule dependency analysis

**Status:** todo

When storing a rule:

- parse body predicates
- record dependencies
- support invalidation / optimization later

### D4. Rule management API

**Status:** todo

Commands:

- `add_rule`
- `list_rules`
- `get_rule`
- `delete_rule`
- `explain_rule`

## Workstream E: Query Planning / Optimization

### E1. Predicate ordering

**Status:** todo

Before WAM compile or during call scheduling:

- prefer predicates with bound/indexed arguments
- delay high-cardinality scans
- place `prop/3` after node/edge binding when possible
- place `similar_to/search` according to selectivity and binding state

### E2. Explain plan

**Status:** todo

Add query explain output:

```json
{
  "predicates": [...],
  "indexes": [...],
  "estimated_cost": ...
}
```

### E3. Limits and cancellation

**Status:** todo

- max recursion depth
- max result count
- timeout
- SSE cancellation / disconnect awareness

## Workstream F: Markdown Knowledge Base Import

### F1. Document/chunk import

**Status:** done_initial

Current import creates:

- document nodes
- markdown chunk nodes
- `contains` edges

### F2. Stable path-based IDs

**Status:** done_initial

Current IDs are path-derived.

### F3. Frontmatter parsing

**Status:** todo

Support YAML/TOML frontmatter:

- title
- tags
- aliases
- created/updated dates
- source metadata

### F4. Link extraction

**Status:** todo

Extract Markdown/wiki links:

- `[[Wiki Link]]`
- `[title](path.md)`
- URL links

Generate edges:

- `links_to`
- `mentions`
- `references`

### F5. Entity/relation extraction

**Status:** todo

Support staged extraction:

- regex/rule-based baseline
- optional LLM-assisted extraction
- human-reviewable JSON output

### F6. Embedding import/generation

**Status:** todo

For documents/chunks/entities:

- accept precomputed embeddings
- generate embeddings via configured provider
- store model metadata
- support re-embedding by model version

## Workstream G: HTTP SSE / Streaming API

### G1. SSE endpoint

**Status:** done_initial

Endpoint:

```text
POST /api/sse
```

Current query events:

- `started`
- `chunk`
- `done`
- `error`

### G2. Streaming query executor

**Status:** todo

Avoid collecting all query results before emitting chunks.

Target:

```text
WAMRuntime yields solution -> SSE chunk immediately
```

### G3. Client disconnect handling

**Status:** todo

Stop query execution when client disconnects.

### G4. SSE integration tests

**Status:** todo

Add tests for:

- health
- ordinary command response
- query streaming chunks
- error event

## Workstream H: External Knowledge Base Operations

### H1. Bulk import command

**Status:** todo

Support:

- folder import
- JSONL import
- incremental re-import
- duplicate strategy
- dry-run mode

### H2. Import manifest

**Status:** todo

Persist import runs:

```text
import:{run_id}
import_file:{run_id}:{path}
```

### H3. Incremental update detection

**Status:** todo

Track:

- file mtime
- content hash
- prior node IDs
- deleted files

### H4. Backup/restore for imported KB

**Status:** todo

Needed before large-scale ingestion.

## Workstream I: API / SDK Contracts

### I1. JSON-over-stdio completeness

**Status:** in_progress

Current commands include init/node/edge/query/search/similar/import/export/index_text/embed/index_embedding/config.

Remaining:

- rule management commands
- explain query
- import dry-run
- streaming control not applicable to stdio unless NDJSON multi-response is added

### I2. HTTP JSON completeness

**Status:** in_progress

Current:

- `/api`
- `/api/sse`
- `/health`

Remaining:

- route tests
- structured error consistency
- SSE cancellation

### I3. TypeScript SDK updates

**Status:** todo

Reflect new commands:

- rules
- markdown import
- SSE query streaming
- explain query

## Workstream J: Acceptance / Verification Matrix

### Required green checks before declaring done

```bash
scripts/check-rust.sh
```

### Functional acceptance tests still needed

- WAM-backed facts/rules parity tests
- DB index-backed fact provider tests
- persistent rule reload tests across process boundary
- Markdown folder import against `~/workspace/docs/wiki/content/`
- BM25 + graph + property + temporal + embedding mixed queries
- SSE chunk streaming tests
- source-size check in CI
- clippy `too_many_lines=30` in CI

## Current Immediate Next Steps

1. Finish deleting/cleaning legacy WAM references. **Mostly done.**
2. Split oversized source files to satisfy file line limits.
3. Refactor long functions to satisfy `too_many_lines=30`.
4. Extract current `PrologEngine` into temporary interpreter modules.
5. Introduce `FactProvider` trait.
6. Start new WAM module with minimal compiler/runtime.
7. Wire WAM runtime to DB-backed `FactProvider`.

## Definition of Done for v2

v2 is done only when:

- `scripts/check-rust.sh` passes.
- Main query path uses WAM runtime or explicitly documented transitional interpreter fallback.
- DB content is exposed through FactProvider as the Prolog fact space.
- Rules persist and are indexed by predicate/arity/dependencies.
- Markdown KB import supports document/chunk/link extraction.
- SSE can stream query chunks without collecting all results first.
- Integration tests cover stdio and HTTP/SSE paths.
