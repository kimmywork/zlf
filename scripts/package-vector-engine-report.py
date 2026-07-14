#!/usr/bin/env python3
"""Package a frozen vector-engine run into zlf-benchmark-report-v1."""

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
    raw = json.loads(args.raw.read_text())
    manifest = json.loads((args.dataset / "manifest.json").read_text())
    report = new_report(
        dataset={
            "name": "frozen-vector-search", "version": manifest["schema"], "tier": "100k",
            "manifest": str(args.dataset / "manifest.json"), "checksums": manifest["files"],
            "license": {"status": "generated", "source": manifest["algorithm"]},
            "judgments": "100 byte-identical self-query document IDs",
        },
        configuration={
            "limits": {
                "documents": raw["documents"], "queries": raw["queries"],
                "candidate_limit": raw["documents"], "page_size": raw["documents"],
                "max_pages": 1, "answer_limit": 100, "timeout_ms": 3_600_000,
                "retry_limit": 0,
            },
            "backend": raw["backend"], "dimension": raw["dimension"],
            "metric": raw["metric"], "normalized": raw["normalized"],
            "top_k": [10, 100], "filters": ["none", "group_10", "group_100"],
        },
        phases={"backend_build_ms": raw["build_ms"], "reopen_ms": raw["reopen_ms"]},
        metrics={
            "workloads": raw["workloads"], "fresh_reader_top10": raw["fresh_reader_top10"],
            "correctness": {"self_queries": raw["self_queries_checked"], "top1_correct": raw["self_top1_correct"]},
            "resources": {"index_bytes": raw["index_bytes"], "peak_rss_bytes": raw["peak_rss_bytes"]},
        },
    )
    report["claim_scope"] = "retrieval-only single-process sequential exact-vector baseline; no Ollama or HTTP"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(args.output)


if __name__ == "__main__":
    main()
