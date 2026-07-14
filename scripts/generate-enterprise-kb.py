#!/usr/bin/env python3
"""Generate deterministic EnterpriseKB graph/ACL/retrieval fixtures."""

import argparse
import hashlib
import json
import shutil
from pathlib import Path

TOPICS = 64
GROUPS = 8
USERS = 32


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_jsonl(path: Path, rows):
    with path.open("w", encoding="utf-8") as stream:
        for row in rows:
            stream.write(json.dumps(row, sort_keys=True) + "\n")


def generate(output: Path, count: int, seed: str):
    tier = output / f"v1-{count // 1000}k"
    if tier.exists():
        shutil.rmtree(tier)
    tier.mkdir(parents=True)
    documents = []
    for index in range(count):
        topic = index % TOPICS
        group = ((index // TOPICS) * 5 + topic * 3) % GROUPS
        active = index % 10 != 0
        documents.append({
            "_id": f"doc-{index:06d}",
            "topic": f"topic{topic:02d}",
            "access_group": f"group-{group:02d}",
            "active": active,
            "body": f"enterprise knowledge topic{topic:02d} policy record{index:06d}",
            "valid_from": "2025-01-01T00:00:00Z",
            "valid_to": "2027-01-01T00:00:00Z" if active else "2025-06-01T00:00:00Z",
        })
    users = [{"_id": f"user-{index:02d}", "group": f"group-{index % GROUPS:02d}"} for index in range(USERS)]
    queries = []
    oracle = []
    for index in range(128):
        topic = f"topic{index % TOPICS:02d}"
        user = users[index % USERS]
        query_id = f"query-{index:03d}"
        queries.append({"_id": query_id, "text": topic, "user": user["_id"], "at": "2026-01-01T00:00:00Z"})
        relevant = [row["_id"] for row in documents if row["topic"] == topic and row["access_group"] == user["group"] and row["active"]]
        oracle.append({"query_id": query_id, "relevant": relevant})
    files = {"documents.jsonl": documents, "users.jsonl": users, "queries.jsonl": queries, "oracle.jsonl": oracle}
    for name, rows in files.items():
        write_jsonl(tier / name, rows)
    manifest = {
        "schema": "zlf-enterprise-kb-v1", "seed": seed, "documents": count,
        "users": USERS, "groups": GROUPS, "topics": TOPICS, "queries": len(queries),
        "acl_rule": "allowed(U,D) :- property(U,group,G), property(D,access_group,G)",
        "temporal_filter": "ValidityStore valid_at at 2026-01-01T00:00:00Z",
        "permission_mutation": {"user": "user-00", "from": "group-00", "to": "group-01"},
        "files": {name: sha256(tier / name) for name in files},
    }
    (tier / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(tier)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("data/benchmarks/enterprise-kb"))
    parser.add_argument("--documents", type=int, nargs="+", default=[1000, 10000])
    parser.add_argument("--seed", default="zlf-enterprise-kb-v1")
    args = parser.parse_args()
    for count in args.documents:
        if count < 1000 or count % 1000:
            parser.error("document counts must be positive multiples of 1000")
        generate(args.output.resolve(), count, args.seed)


if __name__ == "__main__":
    main()
