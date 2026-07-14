import json
import subprocess
import tempfile
import unittest
from pathlib import Path


class VectorSearchDatasetTests(unittest.TestCase):
    def test_generator_matches_frozen_binary_golden_and_reuses_it(self):
        script = Path(__file__).resolve().parents[1] / "prepare-vector-search-dataset.py"
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "vectors"
            command = [
                "python3", str(script), "--output", str(output),
                "--documents", "8", "--queries", "4", "--self-queries", "2",
                "--dimension", "4", "--seed", "123", "--batch-size", "3",
            ]
            subprocess.run(command, check=True, capture_output=True, text=True)
            manifest = json.loads((output / "manifest.json").read_text())
            self.assertEqual(manifest["files"], {
                "document-groups.u16le": "bddaf26cdbfaa91ff80ccdb95b84263c5b4808b3cedd069967ac5fd865c4e848",
                "documents.f32le": "996fa34dcaea5d9a007ff73818dbe32965a8cddcc7df47b3b76a928d22733aea",
                "queries.f32le": "b010225bb7d76f9bf5827de90e032e35303d66bbb516120221a806aeb42cdc17",
                "self-query-document-ids.u32le": "2be2196b9c19b0913b11d708d2550cdf0f5b0106c4ae0eec2aa07d2b243c7268",
            })
            second = subprocess.run(command, check=True, capture_output=True, text=True)
            self.assertIn("verified existing immutable dataset", second.stdout)


if __name__ == "__main__":
    unittest.main()
