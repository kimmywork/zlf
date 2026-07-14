#!/usr/bin/env python3
"""Run the bounded EnterpriseKB benchmark and emit the shared report schema."""

import argparse
import json
import os
import subprocess
from pathlib import Path

from benchmark_contract import (
    CheckpointStore,
    canonical_hash,
    mark_failed,
    new_report,
    sha256_file,
    validate_dataset_manifest,
)


def base_report(manifest, dataset, limits):
    return new_report(
        dataset={
            "name": "enterprise-kb",
            "version": "v1",
            "tier": f"{manifest['documents'] // 1000}k",
            "manifest": os.path.relpath(dataset / "manifest.json", Path.cwd()),
            "checksums": manifest["files"],
            "license": {"status": "generated", "source": "zlf deterministic generator"},
            "judgments": "independent generated ACL/temporal oracle",
        },
        configuration={
            "limits": limits,
            "backend": "Tantivy BM25, WAM graph rule, ordered RocksDB validity",
            "graph_rule": manifest["acl_rule"],
            "temporal": manifest["temporal_filter"],
            "ingestion": "zlf bulk fact pack",
        },
    )


def execute(args, manifest, checkpoint):
    cached = checkpoint.evidence("benchmark")
    if cached is not None and not args.force:
        return cached["raw"]
    result = subprocess.run(
        [str(args.binary), str(args.dataset)],
        check=True,
        capture_output=True,
        text=True,
        timeout=args.timeout_seconds,
    )
    raw = json.loads(result.stdout)
    checkpoint.complete("benchmark", {"raw": raw})
    return raw


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("dataset", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--binary", type=Path, default=Path("target/release/examples/enterprise_kb_h6_benchmark")
    )
    parser.add_argument("--timeout-seconds", type=int, default=1800)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()
    args.dataset = args.dataset.resolve()
    manifest_path = args.dataset / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    validate_dataset_manifest(manifest, args.dataset, True)
    limits = {
        "documents": manifest["documents"], "queries": manifest["queries"],
        "candidate_limit": 256, "page_size": 256, "max_pages": 1,
        "answer_limit": 10, "timeout_ms": args.timeout_seconds * 1000, "retry_limit": 0,
    }
    report = base_report(manifest, args.dataset, limits)
    identity = {
        "manifest_sha256": sha256_file(manifest_path),
        "binary_sha256": sha256_file(args.binary),
        "limits": limits,
        "runner": canonical_hash({"schema": "enterprise-kb-runner-v1"}),
    }
    checkpoint = CheckpointStore(args.dataset / ".zlf-benchmark-checkpoint.json", identity)
    try:
        raw = execute(args, manifest, checkpoint)
        report["phases"] = {
            "combined_build_ms": raw["build_ms"],
            **{f"build_{name}_ms": value for name, value in raw["build_phases_ms"].items()},
            "independent_oracle_ms": raw["query_oracle_ms"],
        }
        report["metrics"] = {
            "query_latency": raw["query_latency"], "candidates": raw["candidate_counts"],
            "filter": raw["filter"], "quality": raw["quality"],
            "correctness": raw["correctness"],
            "resources": {"disk_bytes": raw["disk_bytes"], "peak_rss_bytes": raw["peak_rss_bytes"]},
        }
        report["claim_scope"] = "generated-oracle initial-build composition and scale; mutation files are prepared but not exercised by this run"
    except Exception as error:
        report = mark_failed(report, "benchmark", "execution_failure", error)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
        raise
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(args.output)


if __name__ == "__main__":
    main()
