# Scope Map: Roadmap Stage 9 and Advanced Runtime

## Roadmap reconciliation

| Roadmap item | Existing baseline | New-track disposition |
|---|---|---|
| Proof terms | Delivered in kernel Stage 5 | Maintain; add measured limits only |
| Deterministic tabling | Delivered in Stage 6 | Maintain; not full SLG |
| Incremental tabling | Selective invalidation delivered in Stage 7 | Add true insert/delete delta maintenance |
| Predicate pushdown | Call-time bound provider and query plans delivered in Stage 8 | Extend with declarations, conservative inference, and cursors |
| Virtual address/lazy loading | Typed call-time lazy provider boundary delivered; design spike found no current need for persistent heap pointers | Benchmark gate; do not implement speculatively |
| GC | Roots inventoried; hot tables bounded/evicting; proof is opt-in | Stress first, then query arenas/cursors/GC by measured cause |
| Stratified NAF | `\+/1` exists, no static stratification guarantee | First semantic slice |
| Order-sorted types | Not delivered | Optional after semantics/memory foundations |
| CLP(B)/CLP(FD) | Not delivered | Separate optional modules, CLP(B) before CLP(FD) |
| Probabilistic logic | Proof facade exists | Meta/facade module only |
| MIL | WAM/rule store exists | Bounded offline tool/facade only |
| WFS | Not delivered | Late milestone requiring fuller SLG machinery |
| Predicate closures | `call/1..8` exists, no general closure system | Research spike only |
| AC/linear logic | Not delivered | Research/deferred |
| Parallel queries | Not delivered | Deployment-level research, never kernel OR-parallel by default |

## Delivery sequence

1. **S0 — Baselines and semantic contracts**: conformance corpus, memory/performance harnesses, feature flags, stable result contracts.
2. **S1 — Stratified negation**: signed dependency graph, SCC/strata analysis, diagnostics, mutation refresh.
3. **S2 — Modes and storage cursors**: declarations, conservative rule-level inference, plan visibility, bounded provider iteration.
4. **S3 — Memory lifecycle**: proof limits, query arenas/safe points, retention evidence, collector only if justified.
5. **S4 — Delta table maintenance**: insert/delete deltas, recursive propagation, fallback and metrics.
6. **S5 — Order-sorted types**: optional declaration/type-checking layer.
7. **S6 — CLP(B)**: trailed Boolean constraint store.
8. **S7 — CLP(FD)**: finite-domain propagation and labeling.
9. **S8 — Probabilistic proof facade**: metadata and proof aggregation semantics.
10. **S9 — MIL tool**: bounded candidate generation/validation and review queue.
11. **S10 — WFS**: delayed negation and three-valued tables.
12. **S11 — Research spikes**: closures, AC/linear logic, query concurrency.

S1–S4 are production-foundation candidates. S5–S10 are independently approved optional modules and need not all ship for the parent track to produce value.

## Dependency map

```text
S0 -> S1 stratified NAF ---------------------> S10 WFS
 |      |                                         ^
 |      +-> signed dependency persistence --------|
 |
 +-> S2 modes/cursors -> S3 memory lifecycle
 |          |
 |          +-> storage-scale evidence
 |
 +-> Stage 7 dependency baseline -> S4 delta tables -> S10 WFS
 |
 +-> Stage 5 proof baseline -> S8 probability -> S9 MIL
 |
 +-> S5 order types (independent optional)
 +-> S6 CLP(B) -> S7 CLP(FD)
```

## Explicit boundaries

- No source adapter owns RocksDB private key schemas.
- No optional logic module introduces a second term representation or unifier.
- No probability or MIL logic is inserted into ordinary query execution.
- No WFS state is serialized as the existing positive-table format.
- No virtual-address rewrite begins without a benchmark, migration plan, and rollback plan.
- No generated stress databases, reports, or local datasets are committed unless curated under `docs/track/.../research/`.
