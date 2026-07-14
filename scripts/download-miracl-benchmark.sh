#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOPICS_DIR="${ZLF_MIRACL_DIR:-${ROOT_DIR}/data/miracl}"
CORPUS_DIR="${ZLF_MIRACL_CORPUS_DIR:-${ROOT_DIR}/data/miracl-corpus}"
MIRACL_REVISION="${ZLF_MIRACL_REVISION:-main}"
CORPUS_REVISION="${ZLF_MIRACL_CORPUS_REVISION:-main}"

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

# Topics and official judgments. The English train qrels reproduces the
# earlier manual download; Stage 06 quality evaluation uses the dev files.
run hf download miracl/miracl \
  --repo-type dataset \
  --revision "${MIRACL_REVISION}" \
  --include \
  '**/qrels.miracl-v1.0-en-train.tsv' \
  '**/qrels.miracl-v1.0-en-dev.tsv' \
  '**/topics.miracl-v1.0-en-dev.tsv' \
  '**/qrels.miracl-v1.0-zh-dev.tsv' \
  '**/topics.miracl-v1.0-zh-dev.tsv' \
  --local-dir "${TOPICS_DIR}"

# Only shard 0 is downloaded. The preparation step will retain dev queries
# whose complete positive-judgment set exists in this bounded corpus pool.
run hf download miracl/miracl-corpus \
  --repo-type dataset \
  --revision "${CORPUS_REVISION}" \
  --include \
  'miracl-corpus-v1.0-en/docs-0.jsonl.gz' \
  'miracl-corpus-v1.0-zh/docs-0.jsonl.gz' \
  --local-dir "${CORPUS_DIR}"

if [[ "${ZLF_DATASET_DRY_RUN:-0}" != "1" ]]; then
  for language in en zh; do
    test -f "${TOPICS_DIR}/miracl-v1.0-${language}/qrels/qrels.miracl-v1.0-${language}-dev.tsv"
    test -f "${TOPICS_DIR}/miracl-v1.0-${language}/topics/topics.miracl-v1.0-${language}-dev.tsv"
    test -f "${CORPUS_DIR}/miracl-corpus-v1.0-${language}/docs-0.jsonl.gz"
  done
  echo "MIRACL en/zh benchmark sources are ready under ${TOPICS_DIR} and ${CORPUS_DIR}"
fi
