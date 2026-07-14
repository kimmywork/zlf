# Stage 05 implementation progress v8

## Increment

H6b delivers deterministic EnterpriseKB-v1 generation and 1K/10K graph/rule/temporal/filter/mutation evidence.

## Delivered

- Added `scripts/generate-enterprise-kb.py` with fixed seed, 64 topics, eight groups, 32 users, 128 queries, validity intervals, permission mutation, independent oracle, and file checksums.
- Added `enterprise_kb_h6_benchmark` release example.
- Builds canonical graph properties, a real Tantivy BM25 corpus, and generation-scoped `ValidityStore` records.
- Filters bounded BM25 candidates through validity lookup and an ordinary persisted Prolog ACL rule.
- Compares bounded answer order with an independent full-ranking ACL/temporal oracle.
- Exercises permission mutation and table dependency invalidation.
- Reports candidate counts, selectivity, graph/temporal rejections, p50/p95/p99, peak materialized answers, RSS, disk, exact-query count, precision, and stale-result count.

## Finding and fix

The first 10K run measured approximately 2.9 s p99 because bound canonical `property/3` rule goals fell back to whole-storage fact materialization. Added direct node/edge lookup for `property(Entity, Key, Value)` when entity and key are bound. Property behavior was split into `storage_property.rs` to preserve the Rust source-size policy.

After pushdown, 10K p99 was 4.71 ms with the same 128/128 independent-oracle result.

## Evidence

- `research/enterprise-kb-h6-1k-2026-07-14.json`
- `research/enterprise-kb-h6-10k-2026-07-14.json`
- `research/enterprise-kb-h6-local-2026-07-14.md`

## Results

- 1K: 128/128 exact, 0 stale results, p99 1.05 ms, 158 MiB RSS, 4.11 MiB disk.
- 10K: 128/128 exact, 0 stale results, p99 4.71 ms, 257 MiB RSS, 38.4 MiB disk.
- Permission mutation invalidated table dependencies at both tiers.
- Candidate and answer limits remained 256 and 10.

## Verification

```bash
python3 scripts/generate-enterprise-kb.py
cargo fmt --all
python3 scripts/check-rust-size.py
cargo test -p zlf-prolog --test storage_wam_provider
cargo clippy -p zlf-prolog -p zlf-query --all-targets -- \
  -D warnings -W clippy::too_many_lines
cargo build --release -p zlf-query --example enterprise_kb_h6_benchmark
target/release/examples/enterprise_kb_h6_benchmark \
  data/benchmarks/enterprise-kb/v1-1k
target/release/examples/enterprise_kb_h6_benchmark \
  data/benchmarks/enterprise-kb/v1-10k
git diff --check
```

## Next

H6 is complete with distinct real-quality SciFact evidence and generated-oracle EnterpriseKB composition evidence. Proceed to H7 cumulative review, full quality gates, Stage 05 delivery record, and acceptance decision.
