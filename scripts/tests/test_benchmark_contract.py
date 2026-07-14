import json
import math
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from benchmark_contract import (  # noqa: E402
    BenchmarkLimits,
    CheckpointStore,
    ContractError,
    canonical_hash,
    mark_failed,
    new_report,
    percentile,
    ranking_metrics,
    sha256_file,
    validate_dataset_manifest,
    validate_report,
)


class BenchmarkContractTests(unittest.TestCase):
    def limits(self):
        return {
            "documents": 1000,
            "queries": 100,
            "candidate_limit": 100,
            "page_size": 10,
            "max_pages": 10,
            "answer_limit": 10,
            "timeout_ms": 1000,
            "retry_limit": 3,
        }

    def report(self):
        return {
            "schema": "zlf-benchmark-report-v1",
            "run": {
                "commit": "abc",
                "dirty": False,
                "created_at": "2026-07-14T00:00:00Z",
                "machine": {},
            },
            "dataset": {
                "name": "fixture",
                "version": "v1",
                "tier": "1k",
                "checksums": {"corpus": "a" * 64},
                "license": {"status": "generated"},
            },
            "configuration": {"limits": self.limits()},
            "phases": {},
            "metrics": {},
        }

    def test_limits_reject_unbounded_or_incompatible_shapes(self):
        BenchmarkLimits.from_dict(self.limits())
        for key, value in [
            ("documents", 100001),
            ("candidate_limit", 0),
            ("answer_limit", 101),
            ("page_size", 101),
        ]:
            invalid = self.limits()
            invalid[key] = value
            with self.assertRaises(ContractError):
                BenchmarkLimits.from_dict(invalid)

    def test_manifest_verifies_safe_paths_and_checksums(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "corpus.jsonl").write_text("{}\n")
            manifest = {
                "schema": "fixture-v1",
                "seed": "fixed",
                "files": {"corpus.jsonl": sha256_file(root / "corpus.jsonl")},
            }
            validate_dataset_manifest(manifest, root, True)
            (root / "corpus.jsonl").write_text("changed")
            with self.assertRaisesRegex(ContractError, "checksum mismatch"):
                validate_dataset_manifest(manifest, root, True)
            manifest["files"] = {"../secret": "a" * 64}
            with self.assertRaisesRegex(ContractError, "unsafe"):
                validate_dataset_manifest(manifest)

    def test_report_requires_provenance_limits_license_and_finite_metrics(self):
        report = self.report()
        validate_report(report)
        report["metrics"] = {"latency": math.inf}
        with self.assertRaisesRegex(ContractError, "non-finite"):
            validate_report(report)
        report = self.report()
        report["dataset"]["license"] = {"status": "guessed"}
        with self.assertRaisesRegex(ContractError, "license"):
            validate_report(report)

    def test_partial_failure_report_is_structured_and_redacted(self):
        value = self.report()
        report = new_report(
            value["dataset"], value["configuration"], run=value["run"]
        )
        failed = mark_failed(
            report, "embedding", "provider_failure", RuntimeError("secret source text")
        )
        validate_report(failed)
        self.assertEqual(failed["failure"]["phase"], "embedding")
        self.assertNotIn("secret source text", json.dumps(failed))

    def test_checkpoint_is_atomic_resumable_and_identity_scoped(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "checkpoint.json"
            store = CheckpointStore(path, {"dataset": "a", "model": "m1"})
            store.complete("embedding", {"documents": 10})
            reopened = CheckpointStore(path, {"dataset": "a", "model": "m1"})
            self.assertEqual(reopened.evidence("embedding"), {"documents": 10})
            with self.assertRaisesRegex(ContractError, "identity mismatch"):
                CheckpointStore(path, {"dataset": "a", "model": "m2"})
            self.assertEqual(json.loads(path.read_text())["identity_hash"], canonical_hash({"dataset": "a", "model": "m1"}))

    def test_percentiles_and_quality_match_hand_calculated_fixture(self):
        self.assertEqual(percentile([5, 1, 4, 2, 3], 50), 3)
        metrics = ranking_metrics(["d2", "d1", "d3"], {"d1": 1, "d3": 1})
        self.assertEqual(metrics["reciprocal_rank"], 0.5)
        self.assertEqual(metrics["recall_at_10"], 1.0)
        self.assertEqual(metrics["recall_at_100"], 1.0)
        self.assertGreater(metrics["ndcg_at_10"], 0.6)
        self.assertLess(metrics["ndcg_at_10"], 1.0)


if __name__ == "__main__":
    unittest.main()
