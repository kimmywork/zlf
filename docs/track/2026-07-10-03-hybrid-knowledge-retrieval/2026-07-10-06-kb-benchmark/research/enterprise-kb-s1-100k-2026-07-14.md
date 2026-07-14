# EnterpriseKB S1 initial-build 100K baseline — 2026-07-14

## Scope

This is the first bounded 100K combined graph/BM25/validity scale baseline under the shared Stage 06 report contract. It uses generated data and an independent oracle; it is not semantic-quality or security-isolation evidence.

The generator also emits deterministic revise/delete/insert operations and an after-mutation oracle. Those files are checksum-covered but are explicitly not exercised by this initial-build report; lifecycle execution remains the next S1 increment.

## Reproduction

```bash
python3 scripts/generate-enterprise-kb.py --documents 100000
cargo build --release -p zlf-query --example enterprise_kb_h6_benchmark
python3 scripts/run-enterprise-kb-benchmark.py \
  data/benchmarks/enterprise-kb/v1-100k \
  docs/track/2026-07-10-03-hybrid-knowledge-retrieval/\
2026-07-10-06-kb-benchmark/research/enterprise-kb-s1-100k-2026-07-14.json \
  --force
```

The Python runner verifies all input checksums, scopes its checkpoint by manifest/binary/limits identity, invokes the release binary with a finite timeout, and emits `zlf-benchmark-report-v1`.

## Configuration

- 100,000 initial documents.
- 128 fixed topic/user queries.
- 64 topics, eight groups, 32 users.
- Candidate limit 256; answer limit 10.
- Graph ingestion through a deterministic zlf bulk fact pack.
- Tantivy BM25 candidates.
- Ordinary WAM `allowed/2` property rule.
- Ordered RocksDB `ValidityStore` at a fixed instant.

Prepared mutation set:

- 1,000 revisions.
- 500 deletes.
- 500 inserts.

## Result

- Combined initial build: 10.23 s.
  - Graph bulk compile/load/open: 8.54 s.
  - BM25: 1.27 s.
  - Validity: 0.43 s.
- Independent full-ranking oracle: 99.93 ms.
- Query p50/p95/p99: 6.55 / 7.49 / 8.22 ms.
- 128/128 exact filtered top-k queries.
- Zero stale results.
- 10,976 candidates scanned, 85.75 per query.
- Peak materialized answers: 10.
- Peak RSS: approximately 783 MiB.
- Disk: approximately 28.6 MiB.

## Interpretation

The approved 100K ceiling is feasible for the initial combined workload without changing the production runtime or adding an ANN/cursor path. Bulk graph ingestion reduces build time dramatically compared with the earlier per-node 10K fixture while still publishing one canonical rebuild marker. The evidence does not yet accept S1 lifecycle behavior: mutation application, reopen, watermark, retry/stale jobs, and generation rollback remain to be run and independently checked.
