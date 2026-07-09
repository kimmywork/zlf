#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path

NORMAL_MAX = 300
TEST_MAX = 400

ROOTS = [Path("crates")]
EXCLUDE_PARTS = {"target"}


def is_test_file(path: Path) -> bool:
    parts = set(path.parts)
    name = path.name
    return "tests" in parts or name.endswith("_test.rs") or name == "integration_test.rs"


def max_lines_for(path: Path) -> int:
    return TEST_MAX if is_test_file(path) else NORMAL_MAX


def main() -> int:
    failures: list[str] = []
    for root in ROOTS:
        for path in sorted(root.rglob("*.rs")):
            if EXCLUDE_PARTS.intersection(path.parts):
                continue
            line_count = len(path.read_text().splitlines())
            limit = max_lines_for(path)
            if line_count > limit:
                failures.append(f"{path}: {line_count} lines > {limit}")

    if failures:
        print("Rust source size check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
