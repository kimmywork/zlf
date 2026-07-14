#!/usr/bin/env python3
"""Package a frozen hnsw_rs run into zlf-benchmark-report-v1."""

import argparse
import json
from pathlib import Path

from benchmark_contract import new_report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("raw", type=Path)
    parser.add_argument("dataset", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    text = args.raw.read_text()
    raw = json.loads(text[text.index("{"):])
    manifest = json.loads((args.dataset / "manifest.json").read_text())
    report = new_report(
        dataset={
            "name": "frozen-vector-search",
            "version": manifest["schema"],
            "tier": "100k",
            "manifest": str(args.dataset / "manifest.json"),
            "checksums": manifest["files"],
            "license": {"status": "generated", "source": manifest["algorithm"]},
            "judgments": "exact RocksDB top-k plus 100 byte-identical self queries",
        },
        configuration={
            "limits": {
                "documents": raw["documents"],
                "queries": raw["queries"],
                "candidate_limit": 2048,
                "page_size": 100,
                "max_pages": 1,
                "answer_limit": 100,
                "timeout_ms": 3_600_000,
                "retry_limit": 0,
            },
            "backend": raw["backend"],
            "dimension": raw["dimension"],
            "metric": "cosine",
            "normalized": True,
            "parameters": raw["parameters"],
            "ef_search": [128, 256, 512, 1024, 2048],
            "top_k": [10, 100],
            "filters": ["none", "group_10", "group_100"],
            "filter_semantics": "hnsw_rs graph traversal with FilterT eligibility",
        },
        phases={
            "backend_build_ms": raw["build_ms"],
            "reopen_ms": raw["reopen_ms"],
            "exact_oracle_ms": raw["exact_oracle_ms"],
        },
        metrics={
            "workloads": raw["results"],
            "correctness": {
                "self_queries": raw["self_queries_checked"],
                "top1_correct": raw["self_top1_correct"],
            },
            "canonical_id_mapping": raw["canonical_id_mapping"],
            "lifecycle": raw["lifecycle"],
            "resources": {
                "index_bytes": raw["index_bytes"],
                "peak_rss_bytes": raw["peak_rss_bytes"],
            },
        },
    )
    report["claim_scope"] = (
        "retrieval-only single-process sequential ANN experiment against exact RocksDB; "
        "immutable generation rebuilds; no Ollama, HTTP, or production cutover"
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(args.output)


if __name__ == "__main__":
    main()
