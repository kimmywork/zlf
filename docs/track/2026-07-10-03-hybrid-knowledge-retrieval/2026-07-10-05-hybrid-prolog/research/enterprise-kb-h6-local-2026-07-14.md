# EnterpriseKB H6 local composition baseline — 2026-07-14

## Scope

This is a generated-oracle correctness and local-scale workload, not a real semantic quality benchmark. It exercises BM25 candidate ranking followed by bounded temporal and graph/rule filters through the existing `ZlfDatabase`/WAM path. The ACL rule is ordinary Prolog over persisted graph properties:

```prolog
allowed(User, Document) :-
    property(User, group, Group),
    property(Document, access_group, Group).
```

Validity is checked through the generation-scoped `ValidityStore` at `2026-01-01T00:00:00Z`. An independent generator oracle computes authorized active documents.

## Reproduction

```bash
python3 scripts/generate-enterprise-kb.py

cargo run --release -p zlf-query --example enterprise_kb_h6_benchmark -- \
  data/benchmarks/enterprise-kb/v1-1k
cargo run --release -p zlf-query --example enterprise_kb_h6_benchmark -- \
  data/benchmarks/enterprise-kb/v1-10k
```

Both tiers use 128 fixed topic/user queries, candidate limit 256, answer limit 10, eight groups, 32 users, and 64 topics. The generator records checksums in each tier manifest.

## Results

| tier | docs | build | query p50/p95/p99 | RSS | disk |
|---|---:|---:|---:|---:|---:|
| 1K | 1,000 | 4.29 s | 0.81 / 0.97 / 1.05 ms | 158 MiB | 4.11 MiB |
| 10K | 10,000 | 166.75 s | 4.25 / 4.64 / 4.71 ms | 257 MiB | 38.4 MiB |

The build includes graph ingestion, BM25 index construction, and validity index construction. Query latency excludes one-time full-ranking oracle generation, which was 0.99 ms at 1K and 9.36 ms at 10K.

### 1K

- 2,000 candidates scanned, 15.625 per query on average.
- 228 answers, 11.4% candidate selection rate.
- 1,572 graph rejections and 200 temporal rejections.
- 128/128 bounded results matched the independent ACL/temporal ordering oracle.
- Answer relevance precision was 1.0.
- Stale-result count was 0.
- Permission mutation invalidated the table dependency.

### 10K

- 10,976 candidates scanned, 85.75 per query on average.
- 1,280 answers, 11.66% candidate selection rate.
- 8,488 graph rejections and 1,208 temporal rejections.
- 128/128 bounded results matched the independent ACL/temporal ordering oracle.
- Answer relevance precision was 1.0.
- Stale-result count was 0.
- Permission mutation invalidated the table dependency.

## Implementation note

The workload exposed that canonical bound `property/3` goals were falling back to full storage materialization. H6 added direct node/edge lookup for bound `property(Entity, Key, Value)` goals in `storage_property.rs`. After this pushdown, 10K query p99 was 4.71 ms rather than approximately 2.9 s in the initial measurement. The existing shortcut `prop_key/2` path remains supported.

## Interpretation

This validates bounded candidate/filter ordering, ordinary WAM rule composition, temporal generation lookup, permission mutation invalidation, and independent correctness at 1K/10K. It does not claim production security isolation or semantic quality. The public SciFact report remains the semantic quality evidence; EnterpriseKB is the composition and scale evidence.
