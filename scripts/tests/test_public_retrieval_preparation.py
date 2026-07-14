import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "prepare-public-retrieval.py"
SPEC = importlib.util.spec_from_file_location("prepare_public_retrieval", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PublicRetrievalPreparationTests(unittest.TestCase):
    def test_subset_is_deterministic_and_preserves_complete_positives(self):
        corpus = {
            f"d{index}": {"_id": f"d{index}", "title": "", "text": f"text {index}"}
            for index in range(20)
        }
        queries = {f"q{index}": {"_id": f"q{index}", "text": f"query {index}"} for index in range(3)}
        qrels = [
            ("q0", "d0", 1), ("q0", "d1", 1), ("q0", "d2", 0),
            ("q1", "d3", 1), ("q2", "missing", 1),
        ]
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "subset"
            arguments = (output, corpus, queries, qrels, 3, 10, "fixed-seed", {"dataset": "fixture"})
            MODULE.prepare_subset(*arguments, hard_negatives=True)
            first = json.loads((output / "manifest.json").read_text())
            MODULE.prepare_subset(*arguments, hard_negatives=True)
            second = json.loads((output / "manifest.json").read_text())
            self.assertEqual(first, second)
            self.assertEqual(first["query_count"], 2)
            selected = {json.loads(line)["_id"] for line in (output / "corpus.jsonl").read_text().splitlines()}
            self.assertTrue({"d0", "d1", "d2", "d3"} <= selected)
            self.assertNotIn("missing", selected)


if __name__ == "__main__":
    unittest.main()
