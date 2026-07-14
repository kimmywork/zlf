# Stage 06 implementation progress v6

## Increment

S2 source acquisition is scripted and locally verified for FiQA and bounded MIRACL English/Chinese.

## Delivered

- `scripts/download-fiqa-benchmark.sh`
  - downloads FiQA corpus/query parquet files and official test qrels;
  - supports configurable local directories and revisions;
  - verifies expected files after download.
- `scripts/download-miracl-benchmark.sh`
  - downloads English train qrels for parity with the earlier manual command;
  - downloads English/Chinese dev topics and qrels used for evaluation;
  - downloads only corpus shard 0 for each language;
  - verifies every required file.
- `scripts/download-hybrid-benchmark-datasets.sh` invokes both dataset-specific scripts.
- All scripts support `ZLF_DATASET_DRY_RUN=1` and are idempotent through `hf download`.

## Verification

```bash
bash -n scripts/download-*-benchmark*.sh
ZLF_DATASET_DRY_RUN=1 scripts/download-hybrid-benchmark-datasets.sh
find data/fiqa data/fiqa-qrels data/miracl data/miracl-corpus -type f
```

The expected FiQA parquet/qrels files and MIRACL en/zh dev topics/qrels plus shard-0 corpora are present locally.

## Next

Create deterministic prepared datasets: FiQA 10K/100-query and MIRACL en/zh judged shard pools that retain only queries whose complete positive-document set is available. Emit shared manifests, attribution/license notes, and checksum verification.
