#!/usr/bin/env python3
"""Package a raw public retrieval run into zlf-benchmark-report-v1."""

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
            "name": raw["dataset_name"], "version": args.dataset.name, "tier": "10k",
            "manifest": str(args.dataset / "manifest.json"), "checksums": manifest["files"],
            "license": manifest["license"],
            "judgments": "preserved official qrels on a deterministic bounded corpus",
            "scope": manifest.get("corpus_scope", "relevance-preserving sampled corpus"),
            "language": raw["language"],
        },
        configuration={
            "limits": {
                "documents": raw["documents"], "queries": raw["queries"],
                "candidate_limit": raw["candidate_limit"], "page_size": raw["candidate_limit"],
                "max_pages": 1, "answer_limit": raw["answer_limit"],
                "timeout_ms": 7_200_000, "retry_limit": 0,
            },
            "model": raw["model"], "dimension": raw["dimension"], "metric": raw["metric"],
            "max_input_chars": raw["max_input_chars"], "rrf_k": raw["rrf_k"],
            "backend": "Tantivy BM25 plus exact RocksDB vectors",
        },
        phases={
            "index_build_ms": raw["build_ms"],
            "document_embedding_ms": raw["embedding_ms"],
            "query_embedding_ms": raw["query_embedding_ms"],
        },
        metrics={
            "quality": raw["retrieval_quality"], "retrieval_latency": raw["retrieval_latency"],
            "candidates": raw["candidate_counts"],
            "peak_materialized_answers": raw["peak_materialized_answers"],
            "resources": {"disk_bytes": raw["disk_bytes"], "peak_rss_bytes": raw["peak_rss_bytes"]},
        },
    )
    report["claim_scope"] = (
        "bounded prepared-corpus relevance evidence; not directly comparable to full-corpus leaderboard scores"
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(args.output)


if __name__ == "__main__":
    main()
