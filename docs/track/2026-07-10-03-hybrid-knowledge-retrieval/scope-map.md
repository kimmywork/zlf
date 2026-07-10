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
| 04 | `2026-07-10-04-temporal/` | discovery | event-time `temporal_*` and valid-time `valid_*` indexes |
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
| temporal predicate split | 04, part of 05/06 | decided: `temporal_on/between` for events; `valid_at/valid_overlaps` for validity |
| ANN dependency policy | 03/06 | decided: embedded ANN crates allowed; exact RocksDB oracle/fallback retained |
| embedding model strategy | 03/05/06 | decided: pluggable versioned registry; `bge-m3` dense is the default baseline |
| consistency model | 01–04 | decided: durable eventual default; per-index/version/timeout wait and watermarks |
| chunk ownership | 01–03/06 | decided: explicit adapter chunks plus versioned built-in baseline chunkers |
| indexed fields/profile | 01–05 | decided: immutable versioned `IndexProfile`; explicit production fields; opt-in auto profile |
| property mutation | 01/05 | decided: mutable node/edge property patches; immutable edge relation identity |
| benchmark resources | 06 | decided: current M2 Pro/32 GiB only; smoke 1K–10K and maximum 100K chunks |
| ACL benchmark | 05/06 | decided: graph/Prolog ACL-style filtering, not a complete security subsystem |

## Scope boundary with pending roadmap track

`../2026-07-10-02-roadmap-stage9/` remains pending. This track may use delivered proof, tabling, dependency, and query-planning foundations, but it does not begin stratified negation, CLP, WFS, GC, probability, or MIL work.
