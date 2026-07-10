#!/usr/bin/env python3
"""Run tiered NCBI taxonomy conversion, bulk-load, correctness, and query stress."""

import argparse
import json
import platform
import shutil
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run(command, capture=False):
    started = time.perf_counter()
    result = subprocess.run(command, cwd=ROOT, check=True, text=True, capture_output=capture)
    return result.stdout if capture else "", time.perf_counter() - started


def directory_bytes(path):
    return sum(item.stat().st_size for item in path.rglob("*") if item.is_file())


def tier_run(args, tier, bulk, stress):
    target = args.workdir / str(tier)
    facts = target / "facts"
    pack = target / "taxonomy.zlfpack"
    database = target / "db"
    if target.exists() and not args.reuse:
        shutil.rmtree(target)
    target.mkdir(parents=True, exist_ok=True)
    timings = {}
    if not facts.exists():
        conversion = [
            "python3",
            "scripts/ncbi-taxonomy-to-facts.py",
            str(args.taxdump),
            str(facts),
        ]
        if tier != "full":
            conversion.extend(["--limit", str(tier), "--skip-history"])
        _, timings["dmp_to_pl_seconds"] = run(conversion)
    shards = sorted(str(path) for path in facts.glob("*.pl"))
    if not pack.exists():
        _, timings["pl_to_pack_seconds"] = run([str(bulk), "compile", str(pack), *shards])
    if not (database / "storage").exists():
        _, timings["pack_load_seconds"] = run([str(bulk), "load", str(database), str(pack)])
    stress_command = [str(stress), str(database), args.left, args.right, str(args.iterations)]
    output, timings["query_suite_seconds"] = run(stress_command, capture=True)
    restart_output, timings["restart_query_suite_seconds"] = run(stress_command, capture=True)
    oracle_command = [
        "python3",
        "scripts/ncbi-taxonomy-oracle.py",
        str(args.taxdump),
        args.left,
        args.right,
    ]
    if tier != "full":
        oracle_command.extend(["--limit", str(tier)])
    oracle_output, _ = run(oracle_command, capture=True)
    query = json.loads(output)
    restart_query = json.loads(restart_output)
    oracle = json.loads(oracle_output)
    validate(query, oracle)
    validate(restart_query, oracle)
    return {
        "tier": tier,
        "timings": timings,
        "facts_bytes": directory_bytes(facts),
        "pack_bytes": directory_bytes(pack),
        "database_bytes": directory_bytes(database),
        "query": query,
        "restart_query": restart_query,
        "oracle": oracle,
        "correct": True,
    }


def validate(query, oracle):
    distance = query["taxonomy_distance"]
    if distance["lca"] != oracle["lca"] or distance["distance"] != oracle["distance"]:
        raise RuntimeError(f"distance mismatch: query={distance}, oracle={oracle}")
    measurements = {item["name"]: item for item in query["measurements"]}
    expected = {
        "lineage": len(oracle["left_ancestors"]),
        "descendants": len(oracle["descendants_of_right"]),
        "distance_up_left": len(oracle["left_ancestors"]) + 1,
    }
    for name, rows in expected.items():
        if measurements[name]["rows"] != rows:
            raise RuntimeError(
                f"row count mismatch for {name}: {measurements[name]['rows']} != {rows}"
            )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("taxdump", type=Path)
    parser.add_argument("workdir", type=Path)
    parser.add_argument("--tiers", default="10000,100000")
    parser.add_argument("--left", default="7")
    parser.add_argument("--right", default="6")
    parser.add_argument("--iterations", type=int, default=5)
    parser.add_argument("--reuse", action="store_true")
    parser.add_argument("--release", action="store_true")
    args = parser.parse_args()
    profile = "release" if args.release else "debug"
    build = ["cargo", "build", "-p", "zlf-cli", "--bins"]
    if args.release:
        build.append("--release")
    _, build_seconds = run(build)
    bulk = ROOT / "target" / profile / "zlf-bulk"
    stress = ROOT / "target" / profile / "zlf-taxonomy-stress"
    tiers = [
        value if value == "full" else int(value)
        for value in args.tiers.split(",")
        if value
    ]
    args.workdir.mkdir(parents=True, exist_ok=True)
    report = {
        "format": "zlf-ncbi-taxonomy-stress-v1",
        "environment": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": platform.python_version(),
            "profile": profile,
            "build_seconds": build_seconds,
        },
        "tiers": [tier_run(args, tier, bulk, stress) for tier in tiers],
    }
    report_path = args.workdir / "stress-report.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    print(report_path)


if __name__ == "__main__":
    main()
