#!/usr/bin/env python3
"""Migrate accepted Stage 05 H6 reports into the shared Stage 06 report schema."""

import argparse
import json
from pathlib import Path

from benchmark_contract import REPORT_SCHEMA, validate_report


def limits(report):
    candidate = report.get("candidate_limit", report.get("limits", {}).get("candidate_limit", 1))
    answer = report.get("answer_limit", report.get("limits", {}).get("answer_limit", candidate))
    return {
        "documents": report["documents"],
        "queries": report["queries"],
        "candidate_limit": candidate,
        "page_size": candidate,
        "max_pages": 1,
        "answer_limit": answer,
        "timeout_ms": 3_600_000,
        "retry_limit": 0,
    }


def run_metadata(report):
    return {
        "commit": report.get("commit", "unknown"),
        "dirty": report.get("working_tree_dirty", True),
        "created_at": report["created_at"],
        "machine": report.get("machine", {}),
        "migration": {
            "source_schema": report["schema"],
            "legacy_timeout_default_ms": 3_600_000,
        },
    }


def migrate_scifact(report):
    return {
        "schema": REPORT_SCHEMA,
        "run": run_metadata(report),
        "dataset": {
            "name": "scifact",
            "version": "h6-1000d-100q-v1",
            "tier": "1k",
            "manifest": report["dataset_manifest"],
            "checksums": {
                "corpus.jsonl": report["dataset_corpus_sha256"],
                "queries.jsonl": report["dataset_queries_sha256"],
                "qrels.tsv": report["dataset_qrels_sha256"],
            },
            "license": {
                "status": "pending_upstream_review",
                "source": "BEIR-hosted SciFact archive",
            },
            "judgments": "official SciFact test qrels on a deterministic sampled corpus",
        },
        "configuration": {
            "limits": limits(report),
            "model": report["model"],
            "dimension": report["dimension"],
            "metric": report["metric"],
            "analyzer": report["analyzer"],
            "chunking": report["chunking"],
            "rrf_k": report["rrf_k"],
            "backend": "Tantivy BM25 plus exact RocksDB vectors",
        },
        "phases": {
            "index_build_ms": report["build_ms"],
            "document_embedding_ms": report["embedding_ms"],
            "query_embedding_ms": report["query_embedding_ms"],
        },
        "metrics": {
            "quality": report["retrieval_quality"],
            "retrieval_latency": report["retrieval_latency"],
            "candidates": report["candidate_counts"],
            "resources": {
                "disk_bytes": report["disk_bytes"],
                "peak_rss_bytes": report["peak_rss_bytes"],
            },
            "peak_materialized_answers": report["peak_materialized_answers"],
        },
        "claim_scope": report["quality_interpretation"],
        "legacy": report,
    }


def migrate_enterprise(report):
    tier = "10k" if report["documents"] == 10_000 else "1k"
    return {
        "schema": REPORT_SCHEMA,
        "run": run_metadata(report),
        "dataset": {
            "name": "enterprise-kb",
            "version": "v1",
            "tier": tier,
            "manifest": report["dataset_manifest"],
            "checksums": report["dataset_files"],
            "license": {"status": "generated", "source": "zlf deterministic generator"},
            "judgments": report["oracle"],
        },
        "configuration": {
            "limits": limits(report),
            "graph_rule": report["graph_rule"],
            "temporal": report["temporal"],
            "backend": "Tantivy BM25, WAM graph rule, ordered RocksDB validity",
        },
        "phases": {
            "combined_build_ms": report["build_ms"],
            "independent_oracle_ms": report["oracle_ms"],
        },
        "metrics": {
            "query_latency": report["query_latency"],
            "candidates": report["candidate_counts"],
            "filter": report["filter"],
            "quality": report["quality"],
            "correctness": report["correctness"],
            "resources": {
                "disk_bytes": report["disk_bytes"],
                "peak_rss_bytes": report["peak_rss_bytes"],
            },
        },
        "claim_scope": "generated-oracle composition and local-scale evidence, not semantic quality or a security boundary",
        "legacy": report,
    }


def migrate(path: Path):
    report = json.loads(path.read_text())
    if report.get("schema") == REPORT_SCHEMA:
        validate_report(report)
        return report
    if report.get("schema") == "zlf-scifact-h6-v1":
        migrated = migrate_scifact(report)
    elif report.get("schema") == "zlf-enterprise-kb-h6-v1":
        migrated = migrate_enterprise(report)
    else:
        raise ValueError(f"unsupported H6 schema: {report.get('schema')}")
    validate_report(migrated)
    path.write_text(json.dumps(migrated, indent=2, sort_keys=True) + "\n")
    return migrated


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("reports", nargs="+", type=Path)
    args = parser.parse_args()
    for path in args.reports:
        migrate(path)
        print(path)


if __name__ == "__main__":
    main()
