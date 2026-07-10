---
status: done
scope_type: parent
created: 2026-07-10
version: 1
---

# Delivery Record: NCBI Taxonomy Scale Validation

## Delivered

### Bulk fact pipeline

- Restricted streaming ground-Prolog statement reader/compiler.
- Shared `FactMutation` lowering for normal and bulk writes.
- Canonical deterministic node/edge/version serialization.
- Versioned/checksummed bulk packs with forbidden-key validation.
- Bounded RocksDB `WriteBatch` loading, progress checkpoints, idempotent resume, and completion marker.
- Atomic primary/version/label/property/edge adjacency record plans.
- Prefix-bounded scans, exact property reverse index, BM25 batch writes, and compact predicate metadata.
- `zlf-bulk compile` and `zlf-bulk load` binaries.

### Taxonomy knowledge artifacts

- `scripts/ncbi-taxonomy-to-facts.py`: streaming/sharded converter for nodes, grouped names, parent edges, merged IDs, and deleted IDs.
- `scripts/ncbi-taxonomy-oracle.py`: independent lineage/descendant/LCA/tree-distance oracle.
- Stable `tax_ID` identities, `taxonomy_parent/2`, grouped names, rank/division/genetic-code properties, merged/deleted records.
- Exact reverse scientific-name lookup.

### Deterministic positive tabling

- Typed variant keys and answer dedupe.
- Explicit persisted `:- table p/n.` declarations.
- Recursive SCC grouping and semi-naive delta variants.
- Direct nested completed table subgoals and deterministic cut consumers.
- Common left recursion and normalized binary right-recursive transitive closure.
- Generic call-time `CompositeFactProvider` dispatch with external-answer choice points and proof leaves.
- `TableManager` with bounded/evicting memory hot store and RocksDB completed-table backend.
- Ordered persistent answers, atomic metadata publication, stale format migration, restart reload, and coarse mutation invalidation.
- Metrics for hot/persistent hits, misses, completions, iterations, inserted/duplicate answers, invalidations, and evictions.

### Stress suite

- Tiered `10K`, `100K`, `1M`, and `full` runner.
- Release-profile conversion/compile/load/query measurements.
- Cold computation, hot repeat, and fresh-process persistent-table measurements.
- Direct Prolog lineage, descendants, LCA, and `taxonomic_distance/3` checked against the Python oracle.

## Full-scale evidence

Dataset:

```text
2,857,586 current taxonomy nodes
4,818,129 name rows grouped into taxon properties
99,762 merged IDs
778,611 deleted IDs
6,693,306 generated ground facts
46,776,340 compiled storage records
```

Import:

```text
DMP -> PL:       25.837 s
PL -> bulk pack: 828.820 s
bulk load:       81.439 s
PL size:          1.254 GB
pack size:        8.400 GB
RocksDB size:     1.371 GB
```

Representative full-data results:

```text
Homo sapiens lineage: 31 ancestors, 7.592 ms compute, ~0.05 ms hot
Hominidae descendants: 43, 2.005 ms compute
LCA(tax_9606,tax_9604): tax_9604
Taxonomic distance: 3
Direct LCA/distance: ~0.36 ms compute, ~0.05 ms persistent reload
Wide descendants tax_48479: 26,482 answers
  compute: 353.002 ms
  hot materialization: ~14 ms
  process-restart RocksDB reload/materialization: 21.237 ms
```

Detailed evidence:

- `research/full-stress-findings-v1.md`
- `research/stress-report-full-v1.json`
- `research/direct-query-full-v1.json`

## Acceptance mapping

| Acceptance | Result | Evidence |
|---|---|---|
| DMP -> facts -> pack -> database round trip | pass | `bulk_pack` tests and full run |
| Invalid/non-ground/corrupt input rejected | pass | `bulk_pack` tests |
| Deterministic compilation | pass | `compilation_is_deterministic_for_nested_object_properties` |
| Resumable/idempotent bulk loading | pass | `loader_resumes_from_a_validated_record_checkpoint` |
| Bound indexed storage access | pass | `bound_storage_provider` tests and reverse-name full query |
| Cyclic/left/right/mutual/nested positive tabling | pass | `tabling` test target |
| Memory + RocksDB two-level table store | pass | `table_persistence`, restart integration, full metrics |
| Mutation returns fresh answers | pass | `fact_mutation_invalidates_persistent_answers_before_recompute` |
| Taxonomy LCA and distance match independent oracle | pass | full report/direct query report |
| Full dataset completes | pass | full stress artifacts above |

## Quality gates

Fresh verification:

```text
cargo fmt --all -- --check                                      pass
python3 scripts/check-rust-size.py                              pass
cargo clippy --workspace --all-targets -- -D warnings \
  -W clippy::too_many_lines                                    pass
cargo test --workspace                                         pass
```

Ollama/wiki environment-dependent tests remain ignored/opt-in as documented by the repository.

## Boundaries and follow-up

- “Complete tabling” here means the agreed deterministic explicit positive worklist scope. It does not mean WFS/negation, aggregation, answer subsumption, concurrent SLG producers, or persisted live continuations.
- Stage 7 should replace coarse all-table invalidation with persisted fact/rule/table dependencies and selective stale propagation.
- Bulk compilation is the dominant import cost; an SST backend or faster restricted parser can be evaluated later using the established baseline.

## Final status

Accepted for the NCBI taxonomy scale track. The separate kernel-enhancement parent remains in progress for Stage 7 and later.
