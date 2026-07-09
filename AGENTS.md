# AGENTS.md

Guidance for coding agents working in this repository.

## Project Direction

zlf is a WAM-backed Prolog graph database. The current runtime path is:

```text
zlf-cli / HTTP server / REPL
  -> zlf-query::ZlfDatabase
  -> zlf-prolog::wam::WamRuntime
  -> CompositeFactProvider
       - StorageFactProvider
       - IndexFactProvider
       - compiled rules from StorageRuleStore
  -> RocksDB-backed Storage / indexes
```

Do not reintroduce the removed legacy AST `PrologEngine`, `zlf-api`, or M1/M2/M3 prototype engines as active runtime paths.

## Main Interfaces

- JSON-over-STDIO CLI: `zlf`
- HTTP server: `zlf serve [port]`
- Prolog REPL: `zlf repl [db_path]`

The old `zlf-api` crate is removed.

## Prolog Rules

Rules should be persisted as compiled rule artifacts through `StorageRuleStore`, not as runtime-only AST rules.

Facts can be written from Prolog syntax through `StorageFactWriter` / `IndexedStorageFactWriter`.

Important writable facts:

```prolog
node(Id).
node(Id, [Labels], { props... }).
Label(Id).
edge(Source, Type, Target).
EdgeType(Source, Target).
property(Id, Key, Value).
prop_Key(Id, Value).
```

When writing `Label(Id)` for an existing node, preserve existing properties and merge the label.

## Query Predicates

Use the new predicates. Do not add compatibility aliases for old query names unless explicitly requested.

```prolog
node(Id).
label(Id, Label).
property(Id, Key, Value).
edge(Source, Type, Target).
bm25(Query, Node, Score).
vector_similar(SourceNode, Node, Score).
temporal_on(Date, Node).
temporal_between(StartDate, EndDate, Node).
```

Shortcuts:

```prolog
person(alice).
knows(alice, bob).
prop_name(alice, Name).
```

## Embeddings

Default embedding provider is Ollama:

```text
endpoint: http://localhost:11434
model: bge-m3:latest
dimension: 1024
```

Supported environment overrides:

```bash
ZLF_EMBED_PROVIDER=ollama
ZLF_EMBED_ENDPOINT=http://localhost:11434
OLLAMA_ENDPOINT=http://localhost:11434
ZLF_EMBED_MODEL=bge-m3:latest
ZLF_EMBED_DIMENSION=1024
ZLF_EMBED_API_KEY=...
```

Use `BlockingEmbeddingProvider` only when adapting async `zlf-embed` providers to sync WAM writer hooks.

## Quality Gates

Before delivery, run relevant focused tests and, when feasible, the full gates:

```bash
cargo fmt --all
python3 scripts/check-rust-size.py
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
```

Rust source size policy:

- normal `.rs` files: <= 300 lines
- test `.rs` files: <= 400 lines

If a file exceeds the limit, split by responsibility instead of raising thresholds.

## Local Verification Commands

```bash
cargo test -p zlf-prolog
cargo test -p zlf-query
cargo test -p zlf-cli
cargo test -p zlf-storage
```

Ollama verification requires local Ollama and `bge-m3:latest`:

```bash
cargo test -p zlf-prolog --test ollama_embedding_provider -- --ignored --nocapture
```

Wiki verification requires local wiki markdown data:

```bash
cargo test -p zlf-prolog --test wiki_full_pipeline -- --ignored --nocapture
```

## Notes for Future Work

- Keep `FactProvider` read-side only.
- Write-side indexing belongs in writer hooks or embedding workers.
- Avoid a second Prolog runtime.
- Keep CLI/REPL query behavior aligned with WAM predicates.
