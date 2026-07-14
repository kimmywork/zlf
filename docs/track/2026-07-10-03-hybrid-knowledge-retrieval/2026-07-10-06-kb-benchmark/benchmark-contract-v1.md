# Benchmark Contract v1

## Dataset manifest

A prepared dataset manifest must contain:

```json
{
  "schema": "dataset-specific-version",
  "seed": "fixed-seed",
  "files": {"relative/path": "sha256"}
}
```

Paths must be relative and cannot traverse above the manifest directory. Verification can require every file to exist and match its SHA-256. Dataset-specific fields record source URL, counts, query selection, graph/temporal distribution, and mutations.

## Run identity

Checkpoint reuse is keyed by the canonical SHA-256 of the complete identity input, including at minimum:

```text
dataset manifest checksum
prepared text/chunk fingerprints
model profile/version/revision/dimension
index profiles and generations
seed and tier
all limits
```

A checkpoint with a different identity fails rather than partially reusing embeddings or indexes. Phase updates are written through atomic rename.

## Limits

Every report supplies positive finite values for documents, queries, candidates, page size, page count, answers, and timeout, plus a non-negative retry limit. Additional invariants:

```text
documents <= 100000
answer_limit <= candidate_limit
page_size <= candidate_limit
page_size * max_pages >= answer_limit
```

## Shared report

All Stage 06 reports use `zlf-benchmark-report-v1`:

```json
{
  "schema": "zlf-benchmark-report-v1",
  "run": {
    "commit": "...",
    "dirty": false,
    "created_at": "...",
    "machine": {}
  },
  "dataset": {
    "name": "...",
    "version": "...",
    "tier": "...",
    "checksums": {},
    "license": {"status": "confirmed|pending_upstream_review|manual_only|generated"}
  },
  "configuration": {"limits": {}},
  "phases": {},
  "metrics": {}
}
```

All numeric values must be finite. Phase timings distinguish conversion, ingestion, index publication, document embedding, query embedding, retrieval, and independent oracle work where applicable.

A partial failed run retains completed phases and adds only structured failure metadata:

```text
phase
category
error type
SHA-256 error fingerprint
```

The raw exception text is excluded to avoid leaking source text, credentials, or provider payloads.

## Metric semantics

- Percentiles use sorted nearest-rank-at-or-below indexing consistent with existing Rust fixtures.
- MRR is the first positive-qrel reciprocal rank.
- nDCG@10 uses graded gain `2^relevance - 1`.
- Recall@10/100 divides retrieved positive judgments by all positive judgments for that query.
- Public quality uses preserved official judgments.
- Generated oracle metrics are labeled correctness, not public semantic relevance.
- Embedding generation is never included in retrieval latency.
- Fresh-process/reader is not called OS-cold without filesystem-cache control.

## Commands

```bash
python3 scripts/benchmark_contract.py validate-manifest MANIFEST --verify-files
python3 scripts/benchmark_contract.py validate-report REPORT
python3 -m unittest discover -s scripts/tests -p 'test_benchmark_contract.py' -v
```

The Stage 05 SciFact and EnterpriseKB JSON reports are retained at their existing paths but migrated to this shared envelope. Their original payload remains under `legacy` so accepted evidence and interpretation are not silently rewritten.
