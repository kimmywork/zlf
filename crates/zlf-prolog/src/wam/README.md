# WAM module layout

- `builtins/`: call-time core builtins, native meta-call/dynamic operations, and embedded system rules.
- `engine/`: heap/register/trail/environment/choice-point machinery and instruction execution.
- `compile/`: AST-to-WAM code generation, programs, queries, and rule compilation.
- `providers/`: read-side external relations backed by storage, indexes, graph views, and introspection.
- `storage/`: persistent fact/rule writers, mutation identity, embeddings, and queues.
- `metadata/`: predicates, registries, dependency analysis, and proof metadata.
- `runtime.rs`: top-level runtime assembly and query execution.

`mod.rs` uses explicit `#[path]` declarations so the public Rust module names remain compatible while source files are grouped by responsibility.
