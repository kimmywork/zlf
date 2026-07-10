# Scope Map: Hybrid Knowledge Retrieval

## Delivery topology

```text
01 index identity/lifecycle
 ├──> 02 BM25
 ├──> 03 vectors + embedding pipeline
 └──> 04 temporal semantics/index
          \
02 + 03 + 04 + graph/WAM baseline
 └──> 05 hybrid Prolog retrieval
       └──> 06 general-KB benchmark and stress
```

Stage 06 owns benchmark orchestration, but every implementation stage must deliver focused correctness and performance evidence rather than postponing all verification.

## Stage folders

| ID | Folder | Status | Primary responsibility |
|---|---|---|---|
| 01 | `2026-07-10-01-index-lifecycle/` | discovery | indexed-document identity, mutations, generations, rebuild, observability |
| 02 | `2026-07-10-02-bm25/` | discovery | corpus-normalized lexical ranking and multilingual analysis |
| 03 | `2026-07-10-03-vector-embedding/` | discovery | model-safe vectors, exact oracle, ANN, embedding jobs |
| 04 | `2026-07-10-04-temporal/` | blocked on temporal-model decision | ordered temporal indexes and boundary semantics |
| 05 | `2026-07-10-05-hybrid-prolog/` | discovery | fusion, graph/rule/time joins, bounded provider contracts |
| 06 | `2026-07-10-06-kb-benchmark/` | discovery | public/synthetic datasets, tiered stress, machine-readable reports |

## Shared contracts

All stages inherit:

- one active WAM runtime and existing typed `Term` identity;
- `FactProvider` remains read-side only;
- index mutation originates from canonical storage mutation semantics;
- versioned schemas and deterministic key encoding;
- idempotent/resumable jobs and rebuilds;
- explicit top-k/limits and deterministic tie-breaking;
- independent correctness oracles;
- generated data/indexes remain untracked.

## Cross-stage decisions still required

| Decision | Blocks | Recommended default |
|---|---|---|
| temporal domain model | 04, part of 05/06 | valid-time half-open intervals plus separate event timestamp; confirm with user |
| ANN dependency policy | 03/06 | pluggable backend: exact RocksDB oracle plus measured embedded HNSW candidate |
| consistency model | 01–04 | durable eventual consistency with per-generation watermark; optional synchronous wait |
| chunk ownership | 01–03/06 | ingestion adapter supplies chunks initially; zlf stores stable chunk identity |
| full-tier resources | 06 | discover local limits, then define 10K/100K/1M/full tiers |
| ACL benchmark | 05/06 | model ACL as graph predicates first, no dedicated security subsystem |

## Scope boundary with pending roadmap track

`../2026-07-10-02-roadmap-stage9/` remains pending. This track may use delivered proof, tabling, dependency, and query-planning foundations, but it does not begin stratified negation, CLP, WFS, GC, probability, or MIL work.
