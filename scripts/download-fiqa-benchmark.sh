#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIQA_DIR="${ZLF_FIQA_DIR:-${ROOT_DIR}/data/fiqa}"
QRELS_DIR="${ZLF_FIQA_QRELS_DIR:-${ROOT_DIR}/data/fiqa-qrels}"
FIQA_REVISION="${ZLF_FIQA_REVISION:-main}"
QRELS_REVISION="${ZLF_FIQA_QRELS_REVISION:-main}"

command -v hf >/dev/null 2>&1 || {
  echo "error: Hugging Face CLI 'hf' is required" >&2
  exit 1
}

run() {
  if [[ "${ZLF_DATASET_DRY_RUN:-0}" == "1" ]]; then
    printf ' %q' "$@"
    printf '\n'
  else
    "$@"
  fi
}

run hf download BeIR/fiqa \
  --repo-type dataset \
  --revision "${FIQA_REVISION}" \
  --include 'corpus/*' 'queries/*' 'README.md' \
  --local-dir "${FIQA_DIR}"

run hf download BeIR/fiqa-qrels \
  --repo-type dataset \
  --revision "${QRELS_REVISION}" \
  --include 'test.tsv' 'README.md' \
  --local-dir "${QRELS_DIR}"

if [[ "${ZLF_DATASET_DRY_RUN:-0}" != "1" ]]; then
  test -f "${FIQA_DIR}/corpus/corpus-00000-of-00001.parquet"
  test -f "${FIQA_DIR}/queries/queries-00000-of-00001.parquet"
  test -f "${QRELS_DIR}/test.tsv"
  echo "FiQA benchmark sources are ready under ${FIQA_DIR} and ${QRELS_DIR}"
fi
