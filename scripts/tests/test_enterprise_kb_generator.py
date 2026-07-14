import json
import subprocess
import tempfile
import unittest
from pathlib import Path


class EnterpriseKbGeneratorTests(unittest.TestCase):
    def test_generator_is_deterministic_and_emits_audited_mutations(self):
        script = Path(__file__).resolve().parents[1] / "generate-enterprise-kb.py"
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "first"
            command = [
                "python3", str(script), "--output", str(output),
                "--documents", "1000", "--seed", "fixture-seed",
            ]
            subprocess.run(command, check=True, capture_output=True, text=True)
            tier = output / "v1-1k"
            first = json.loads((tier / "manifest.json").read_text())
            subprocess.run(command, check=True, capture_output=True, text=True)
            second = json.loads((tier / "manifest.json").read_text())
            self.assertEqual(first, second)
            self.assertEqual(first["mutations"], {"revise": 10, "delete": 5, "insert": 5})
            mutations = [json.loads(line) for line in (tier / "mutations.jsonl").read_text().splitlines()]
            self.assertEqual(len(mutations), 20)
            deleted = {row["_id"] for row in mutations if row["kind"] == "delete"}
            after = [json.loads(line) for line in (tier / "oracle-after.jsonl").read_text().splitlines()]
            self.assertTrue(all(deleted.isdisjoint(row["relevant"]) for row in after))
            self.assertEqual(first["files"], second["files"])


if __name__ == "__main__":
    unittest.main()
