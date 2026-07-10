# NCBI Taxonomy Full-Scale Stress Findings v1

## Status and confidence

- **Dataset/import counts:** confirmed from local source files, converter manifest, and bulk manifest.
- **Correctness:** cross-referenced between tabled Prolog output and the independent Python parent-map oracle.
- **Latency/throughput:** confirmed on the local machine for the commands and release build below; not a portable SLA.
- **Tabling scope:** deterministic explicit positive tabling only. Negation/WFS, aggregation, answer subsumption, and persisted live WAM continuations remain out of scope.

## Environment

- Apple M2 Pro, 10 logical CPUs, 32 GiB RAM.
- macOS 15.5 arm64.
- Release build.
- NCBI dump retrieved 2026-07-10.

Source SHA-256:

| File | SHA-256 |
|---|---|
| `nodes.dmp` | `389b62d158909df91660dbdd40b2780b67f43c743b8efb6c15e2d614af2793ae` |
| `names.dmp` | `8df87ffe62d344c797c582756b0a4c28c4dbc560c72db6ee4af180b7e735c5fb` |
| `merged.dmp` | `b6e92eea2e58403f593b806955055757d62bba59af965328f898be3c4f6a5944` |
| `delnodes.dmp` | `eab015e676444bd57faa5f3eec3169d2f5e3b035cc2bf1530f18dd9cc9ed920d` |

Machine-readable output: `stress-report-full-v1.json`.

## Full conversion and bulk loading

```text
DMP -> canonical PL facts:       25.837 s
PL facts -> versioned bulk pack: 828.820 s
Bulk pack -> RocksDB:            81.439 s
```

Artifacts:

```text
canonical facts:     6,693,306
fact shards:         136
storage KV records:  46,776,340
PL size:              1.254 GB
bulk pack size:       8.400 GB
RocksDB size:         1.371 GB
```

The full artifact includes current taxonomy nodes and parent edges plus merged/deleted taxonomy records. The converter groups all names for a taxon into the taxon node, so the 4.8M name rows do not become 4.8M extra graph nodes.

## Correctness workload: Homo sapiens and Hominidae

Inputs:

```text
left  = tax_9606 (Homo sapiens)
right = tax_9604 (Hominidae)
```

Oracle and Prolog agreed on:

```text
left lineage ancestors: 31
right descendants:      43
LCA:                    tax_9604
taxonomy tree distance: 3 edges
```

### First process: table computation

| Workload | Answers | Cold | Hot repeat |
|---|---:|---:|---:|
| scientific name by tax ID | 1 | 0.199 ms | 0.077–0.098 ms |
| lineage | 31 | 7.592 ms | 0.047–0.082 ms |
| descendants | 43 | 2.005 ms | 0.053–0.065 ms |
| distance map, left | 32 | 3.043 ms | 0.054–0.062 ms |
| distance map, right | 29 | 2.690 ms | 0.053–0.059 ms |

Table metrics:

```text
misses:             4
tables completed:   4
iterations:         96
inserted answers:   135
hot hits:           22
persistent hits:    0
```

### Fresh process: RocksDB table reload

| Workload | Answers | Persistent first query | Hot repeat |
|---|---:|---:|---:|
| lineage | 31 | 0.112 ms | 0.043–0.051 ms |
| descendants | 43 | 0.078 ms | 0.048–0.053 ms |
| distance map, left | 32 | 0.069 ms | 0.051–0.054 ms |
| distance map, right | 29 | 0.068 ms | 0.049–0.056 ms |

Metrics confirmed:

```text
persistent hits:    4
hot hits:           22
misses:             0
tables completed:   0
iterations:         0
```

This distinguishes all three paths: compute, persistent cold reload, and hot memory reuse.

## Wide-answer workload

Inputs:

```text
left  = tax_129657
right = tax_48479
```

Correct result:

```text
LCA:       tax_48479
distance:  1
right descendants: 26,482
```

First computation:

```text
descendant answer computation: 353.002 ms
hot answer materialization:      13.7–16.6 ms
inserted table answers:          26,495 total across workloads
```

Fresh-process RocksDB reload:

```text
persistent descendant load/materialization: 21.237 ms
subsequent hot materialization:              13.2–14.2 ms
persistent hits:                             4
recomputation iterations:                    0
```

## Direct Prolog LCA and taxonomy distance

After adding deterministic consumers over the two completed `taxonomy_distance_up/3` tables, the full database returned:

```prolog
? taxonomy_lca(tax_9606, tax_9604, Lca).
% Lca = tax_9604

? taxonomic_distance(tax_9606, tax_9604, Distance).
% Distance = 3
```

First computation after table format migration:

```text
taxonomy_lca/3:       0.361 ms
taxonomic_distance/3: 0.356 ms
```

Fresh-process RocksDB table reload:

```text
taxonomy_lca/3:       0.048 ms
taxonomic_distance/3: 0.049 ms
persistent hits:      6
```

Both values matched the independent Python parent-tree oracle.

## Exact reverse name lookup

On the full RocksDB, the scalar property reverse index resolved:

```prolog
? prop_scientific_name(Taxon, "Homo sapiens").
```

Result:

```text
answers: 1
latency: 0.148–0.968 ms cold depending on RocksDB cache state; about 0.09–0.17 ms warm
Taxon: tax_9606
```

This path uses the exact property index rather than scanning all taxon nodes.

## Intermediate 100K scale

Release profile:

```text
facts:                  172,716
storage records:      1,390,864
DMP -> PL:                1.523 s
PL -> pack:              44.984 s
pack -> RocksDB:          2.255 s
facts size:              51.6 MB
pack size:              296.8 MB
RocksDB size:            56.9 MB
```

A 2,153-answer induced-subset descendant query completed in 22.498 ms cold, about 0.95–1.16 ms hot, and about 2.0 ms from a fresh-process persistent table.

## Operational findings

1. **Two-level tabling works at full dataset scale.** Complete answers are published atomically to RocksDB, reloaded into the hot table on restart, and bounded hot tables can evict/reload complete entries.
2. **Answer materialization remains proportional to answer count.** A 26K answer hot query still costs about 14 ms; caching does not remove JSON/result construction cost.
3. **Bulk compilation dominates import time.** The restricted PL parser and record serialization took about 13m49s, versus 81s for RocksDB loading. Future import optimization should target parser/serialization or an optional sorted SST backend.
4. **The original full pack predates compact predicate-registry metadata.** Process startup in the archived machine report includes legacy registry discovery. New packs emit deduplicated label/property/edge-type metadata; the 10K metadata-enabled restart suite fell to about 0.22s process wall time.
5. **Coarse invalidation is correct but not selective.** Any fact/rule mutation marks all persisted tables stale. Fact/rule/table reverse dependencies remain the next incremental-tabling optimization.

## Known boundaries

- The implementation is complete for the agreed deterministic positive worklist MVP, including variants, SCC grouping, delta propagation, direct nested tabled subgoals, hot memory, RocksDB complete tables, restart loading, limits, eviction, and coarse stale recomputation.
- It is not full SLG/WFS Prolog tabling. Tabled negation, aggregation, answer subsumption, concurrent producers, answer-level support counts, and persistence of live continuations are not implemented.
- Right-recursive binary transitive closure is normalized to the left-recursive delta form for the supported graph pattern; arbitrary impure or generative recursive rule shapes remain rejected or outside the performance guarantee.
