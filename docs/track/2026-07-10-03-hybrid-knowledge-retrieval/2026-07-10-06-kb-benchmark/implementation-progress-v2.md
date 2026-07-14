# Stage 06 implementation progress v2

## Increment

S1a establishes deterministic mutation fixtures, bulk graph ingestion, shared-schema orchestration, and the first 100K combined initial-build baseline.

## Delivered

- EnterpriseKB now generates 1K/10K/100K tiers by default.
- Every tier adds deterministic, checksum-covered:
  - 1% document revisions;
  - 0.5% deletes;
  - 0.5% inserts;
  - after-mutation query oracle.
- Added deterministic generator coverage that reruns a 1K tier and verifies manifests, mutation counts, and deleted-document exclusion.
- Replaced benchmark per-node graph setup with the production bulk fact-pack path and one rebuild event.
- Split graph, BM25, validity, independent-oracle, and query timings.
- Added `run-enterprise-kb-benchmark.py` with:
  - manifest checksum verification;
  - finite timeout;
  - manifest/binary/limit-scoped checkpoint;
  - shared `zlf-benchmark-report-v1` output;
  - structured redacted partial-failure output.
- Completed the 100K initial-build run.

## Result

- Build: 10.23 s total; graph 8.54 s, BM25 1.27 s, validity 0.43 s.
- Query p50/p95/p99: 6.55/7.49/8.22 ms.
- 128/128 exact independent-oracle results.
- Zero stale results.
- 85.75 candidates per query average; peak answers 10.
- Approximately 783 MiB RSS and 28.6 MiB disk.

## Evidence

- `research/enterprise-kb-s1-100k-2026-07-14.json`
- `research/enterprise-kb-s1-100k-2026-07-14.md`

## Verification

```bash
python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v
python3 scripts/generate-enterprise-kb.py --documents 100000
cargo fmt --all -- --check
python3 scripts/check-rust-size.py
cargo clippy -p zlf-query --example enterprise_kb_h6_benchmark -- \
  -D warnings -W clippy::too_many_lines
cargo build --release -p zlf-query --example enterprise_kb_h6_benchmark
python3 scripts/run-enterprise-kb-benchmark.py \
  data/benchmarks/enterprise-kb/v1-100k <report> --force
python3 scripts/benchmark_contract.py validate-report <report>
git diff --check
```

## Remaining S1

Apply and independently verify generated revisions/deletes/inserts, process reopen, minimum-watermark waits, embedding retry/stale suppression, and generation rebuild activation/rollback. The 100K report deliberately labels these as not yet exercised.
