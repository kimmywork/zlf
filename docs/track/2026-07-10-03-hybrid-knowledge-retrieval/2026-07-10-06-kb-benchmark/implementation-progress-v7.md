# Stage 06 implementation progress v7

## Increment

S2 deterministic public retrieval preparation for FiQA and bounded MIRACL English/Chinese.

## Delivered

- Added `scripts/prepare-public-retrieval.py`.
- FiQA output:
  - 10,000 documents;
  - 100 deterministic test queries;
  - 262 preserved official qrels;
  - every positive document retained before hash-ranked distractor sampling.
- MIRACL English shard-0 judged pool:
  - 10,000 documents;
  - all 30 dev queries whose complete positive set exists in shard 0;
  - 95 preserved positive/negative judgments.
- MIRACL Chinese shard-0 judged pool:
  - 10,000 documents;
  - 99 dev queries whose complete positive set exists in shard 0;
  - 510 preserved positive/negative judgments.
- MIRACL preparation retains all available judged negatives for selected queries before adding deterministic distractors.
- Every output contains canonical corpus/query JSONL, qrels TSV, source/output SHA-256 values, selection policy, counts, language/split/scope, and explicit pending license review.
- Added deterministic unit coverage for complete-positive eligibility, hard-negative retention, missing-positive exclusion, and rerun identity.

## Verification

```bash
python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v
python3 scripts/prepare-public-retrieval.py
python3 scripts/benchmark_contract.py validate-manifest <each manifest> --verify-files
# A second full preparation produced identical manifest hashes.
```

## Interpretation

The MIRACL outputs are bounded shard-0 judged pools, not full-corpus MIRACL leaderboard runs. English has only 30 eligible complete-positive dev queries in shard 0; Chinese has 99. Reports must retain this scope label and cannot compare directly with full MIRACL published scores.

## Next

Complete primary-source attribution/license records, then run BM25/exact `bge-m3`/RRF quality on FiQA and MIRACL en/zh using identical prepared judgments. Document embedding time remains separate from retrieval latency.
