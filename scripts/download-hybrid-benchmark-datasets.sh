#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${SCRIPT_DIR}/download-fiqa-benchmark.sh"
"${SCRIPT_DIR}/download-miracl-benchmark.sh"

echo "FiQA and bounded MIRACL en/zh benchmark sources are ready."
