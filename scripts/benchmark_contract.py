#!/usr/bin/env python3
"""Shared contracts and utilities for reproducible zlf benchmark runs."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import subprocess
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

REPORT_SCHEMA = "zlf-benchmark-report-v1"
CHECKPOINT_SCHEMA = "zlf-benchmark-checkpoint-v1"
LICENSE_STATUSES = {"confirmed", "pending_upstream_review", "manual_only", "generated"}


class ContractError(ValueError):
    """Raised when benchmark input is unsafe, stale, or incomplete."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_hash(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()


@dataclass(frozen=True)
class BenchmarkLimits:
    documents: int
    queries: int
    candidate_limit: int
    page_size: int
    max_pages: int
    answer_limit: int
    timeout_ms: int
    retry_limit: int

    @classmethod
    def from_dict(cls, value: dict[str, Any]) -> "BenchmarkLimits":
        try:
            limits = cls(**{field: int(value[field]) for field in cls.__dataclass_fields__})
        except (KeyError, TypeError, ValueError) as error:
            raise ContractError(f"invalid benchmark limits: {error}") from error
        limits.validate()
        return limits

    def validate(self) -> None:
        for name, value in self.__dict__.items():
            if value <= 0 and name != "retry_limit":
                raise ContractError(f"{name} must be positive")
            if name == "retry_limit" and value < 0:
                raise ContractError("retry_limit cannot be negative")
        if self.documents > 100_000:
            raise ContractError("documents exceed the approved 100K local ceiling")
        if self.answer_limit > self.candidate_limit:
            raise ContractError("answer_limit cannot exceed candidate_limit")
        if self.page_size > self.candidate_limit:
            raise ContractError("page_size cannot exceed candidate_limit")
        if self.page_size * self.max_pages < self.answer_limit:
            raise ContractError("page budget cannot reach answer_limit")


def validate_dataset_manifest(
    manifest: dict[str, Any], root: Path | None = None, verify_files: bool = False
) -> None:
    required = {"schema", "seed", "files"}
    missing = required - manifest.keys()
    if missing:
        raise ContractError(f"dataset manifest missing: {sorted(missing)}")
    if not isinstance(manifest["files"], dict) or not manifest["files"]:
        raise ContractError("dataset manifest files must be a non-empty object")
    for name, checksum in manifest["files"].items():
        if Path(name).is_absolute() or ".." in Path(name).parts:
            raise ContractError(f"unsafe dataset path: {name}")
        if not isinstance(checksum, str) or len(checksum) != 64:
            raise ContractError(f"invalid SHA-256 for {name}")
        if verify_files:
            if root is None:
                raise ContractError("root is required when verifying files")
            path = root / name
            if not path.is_file():
                raise ContractError(f"dataset file is missing: {name}")
            actual = sha256_file(path)
            if actual != checksum:
                raise ContractError(f"dataset checksum mismatch for {name}: {actual}")


def new_report(
    dataset: dict[str, Any],
    configuration: dict[str, Any],
    phases: dict[str, Any] | None = None,
    metrics: dict[str, Any] | None = None,
    run: dict[str, Any] | None = None,
) -> dict[str, Any]:
    report = {
        "schema": REPORT_SCHEMA,
        "run": run or capture_run(),
        "dataset": dataset,
        "configuration": configuration,
        "phases": phases or {},
        "metrics": metrics or {},
    }
    validate_report(report)
    return report


def mark_failed(
    report: dict[str, Any], phase: str, category: str, error: BaseException
) -> dict[str, Any]:
    if not phase or not category:
        raise ContractError("failed report requires phase and category")
    failed = json.loads(json.dumps(report))
    failed["failure"] = {
        "phase": phase,
        "category": category,
        "error_type": type(error).__name__,
        "error_fingerprint": hashlib.sha256(str(error).encode()).hexdigest(),
    }
    validate_report(failed)
    return failed


def validate_report(report: dict[str, Any]) -> None:
    if report.get("schema") != REPORT_SCHEMA:
        raise ContractError(f"unsupported report schema: {report.get('schema')}")
    for key in ("run", "dataset", "configuration", "phases", "metrics"):
        if not isinstance(report.get(key), dict):
            raise ContractError(f"report {key} must be an object")
    run = report["run"]
    for key in ("commit", "dirty", "created_at", "machine"):
        if key not in run:
            raise ContractError(f"report run missing {key}")
    dataset = report["dataset"]
    for key in ("name", "version", "tier", "checksums", "license"):
        if key not in dataset:
            raise ContractError(f"report dataset missing {key}")
    status = dataset["license"].get("status") if isinstance(dataset["license"], dict) else None
    if status not in LICENSE_STATUSES:
        raise ContractError(f"invalid dataset license status: {status}")
    BenchmarkLimits.from_dict(report["configuration"].get("limits", {}))
    if "failure" in report:
        failure = report["failure"]
        for key in ("phase", "category", "error_type", "error_fingerprint"):
            if not isinstance(failure.get(key), str) or not failure[key]:
                raise ContractError(f"report failure missing {key}")
        if len(failure["error_fingerprint"]) != 64:
            raise ContractError("invalid failure error fingerprint")
    _validate_finite(report)


def _validate_finite(value: Any, path: str = "report") -> None:
    if isinstance(value, float) and not math.isfinite(value):
        raise ContractError(f"non-finite number at {path}")
    if isinstance(value, dict):
        for key, child in value.items():
            _validate_finite(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _validate_finite(child, f"{path}[{index}]")


def capture_run() -> dict[str, Any]:
    commit = _git(["rev-parse", "HEAD"], "unknown")
    dirty = bool(_git(["status", "--porcelain"], ""))
    return {
        "commit": commit,
        "dirty": dirty,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "machine": {
            "os": platform.platform(),
            "architecture": platform.machine(),
            "cpu": platform.processor() or _sysctl("machdep.cpu.brand_string"),
            "memory_bytes": _memory_bytes(),
        },
    }


def _git(args: list[str], fallback: str) -> str:
    try:
        return subprocess.run(
            ["git", *args], check=True, capture_output=True, text=True, timeout=10
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return fallback


def _sysctl(name: str) -> str:
    try:
        return subprocess.run(
            ["sysctl", "-n", name], check=True, capture_output=True, text=True, timeout=5
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return "unknown"


def _memory_bytes() -> int | None:
    value = _sysctl("hw.memsize")
    if value.isdigit():
        return int(value)
    try:
        pages = os.sysconf("SC_PHYS_PAGES")
        page_size = os.sysconf("SC_PAGE_SIZE")
        return int(pages * page_size)
    except (ValueError, OSError, AttributeError):
        return None


def percentile(values: Iterable[float], percent: int) -> float:
    ordered = sorted(values)
    if not ordered or not 0 <= percent <= 100:
        raise ContractError("percentile requires values and a percentage in [0, 100]")
    return ordered[(len(ordered) - 1) * percent // 100]


def ranking_metrics(ranking: list[str], qrels: dict[str, int]) -> dict[str, float]:
    relevant = {key for key, score in qrels.items() if score > 0}
    reciprocal_rank = next(
        (1.0 / rank for rank, key in enumerate(ranking, 1) if key in relevant), 0.0
    )
    return {
        "reciprocal_rank": reciprocal_rank,
        "ndcg_at_10": _ndcg(ranking, qrels, 10),
        "recall_at_10": _recall(ranking, relevant, 10),
        "recall_at_100": _recall(ranking, relevant, 100),
    }


def _recall(ranking: list[str], relevant: set[str], limit: int) -> float:
    return 0.0 if not relevant else len(set(ranking[:limit]) & relevant) / len(relevant)


def _ndcg(ranking: list[str], qrels: dict[str, int], limit: int) -> float:
    def dcg(scores: Iterable[int]) -> float:
        return sum((2**score - 1) / math.log2(rank + 1) for rank, score in enumerate(scores, 1))
    actual = dcg(qrels.get(key, 0) for key in ranking[:limit])
    ideal = dcg(sorted(qrels.values(), reverse=True)[:limit])
    return 0.0 if ideal == 0 else actual / ideal


class CheckpointStore:
    def __init__(self, path: Path, identity: dict[str, Any]):
        self.path = path
        self.identity_hash = canonical_hash(identity)
        if path.exists():
            self.value = json.loads(path.read_text())
            if self.value.get("identity_hash") != self.identity_hash:
                raise ContractError("checkpoint identity mismatch")
        else:
            self.value = {
                "schema": CHECKPOINT_SCHEMA,
                "identity_hash": self.identity_hash,
                "completed": {},
            }

    def complete(self, phase: str, evidence: dict[str, Any]) -> None:
        if not phase or not isinstance(evidence, dict):
            raise ContractError("checkpoint phase and evidence are required")
        self.value["completed"][phase] = evidence
        self._atomic_write()

    def evidence(self, phase: str) -> dict[str, Any] | None:
        return self.value["completed"].get(phase)

    def _atomic_write(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile("w", dir=self.path.parent, delete=False) as stream:
            json.dump(self.value, stream, indent=2, sort_keys=True)
            stream.write("\n")
            temporary = Path(stream.name)
        temporary.replace(self.path)


def main() -> None:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    manifest = subparsers.add_parser("validate-manifest")
    manifest.add_argument("path", type=Path)
    manifest.add_argument("--verify-files", action="store_true")
    report = subparsers.add_parser("validate-report")
    report.add_argument("path", type=Path)
    args = parser.parse_args()
    value = json.loads(args.path.read_text())
    if args.command == "validate-manifest":
        validate_dataset_manifest(value, args.path.parent, args.verify_files)
    else:
        validate_report(value)
    print(args.path)


if __name__ == "__main__":
    main()
