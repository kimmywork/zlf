#!/usr/bin/env python3
"""Download BEIR SciFact and create a deterministic relevance-preserving H6 subset."""

import argparse
import csv
import hashlib
import json
import shutil
import urllib.request
import zipfile
from pathlib import Path

SOURCE_URL = "https://public.ukp.informatik.tu-darmstadt.de/thakur/BEIR/datasets/scifact.zip"
SOURCE_MD5 = "5f7d1de60b170fc8027bb7898e2efca1"


def digest(path: Path, algorithm: str = "sha256") -> str:
    value = hashlib.new(algorithm)
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def ranked(values, seed: str):
    return sorted(values, key=lambda value: (hashlib.sha256(f"{seed}\0{value}".encode()).digest(), value))


def load_jsonl(path: Path):
    with path.open(encoding="utf-8") as stream:
        return [json.loads(line) for line in stream if line.strip()]


def write_jsonl(path: Path, rows):
    with path.open("w", encoding="utf-8") as stream:
        for row in rows:
            stream.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def safe_extract(archive: Path, destination: Path):
    destination.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive) as bundle:
        root = destination.resolve()
        for member in bundle.infolist():
            target = (destination / member.filename).resolve()
            if root not in target.parents and target != root:
                raise ValueError(f"unsafe archive member: {member.filename}")
        bundle.extractall(destination)


def locate_dataset(raw: Path) -> Path:
    for candidate in [raw / "scifact", raw]:
        if (candidate / "corpus.jsonl").is_file() and (candidate / "queries.jsonl").is_file():
            return candidate
    raise FileNotFoundError("SciFact corpus.jsonl/queries.jsonl were not found after extraction")


def read_qrels(path: Path):
    with path.open(encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        return [
            {
                "query-id": row["query-id"],
                "corpus-id": row["corpus-id"],
                "score": int(row["score"]),
            }
            for row in reader
        ]


def write_qrels(path: Path, rows):
    with path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=["query-id", "corpus-id", "score"], delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def prepare(args):
    output = args.output.resolve()
    raw = output / "raw"
    archive = raw / "scifact.zip"
    raw.mkdir(parents=True, exist_ok=True)
    if not archive.exists():
        print(f"downloading {SOURCE_URL}")
        urllib.request.urlretrieve(SOURCE_URL, archive)
    actual_md5 = digest(archive, "md5")
    if actual_md5 != SOURCE_MD5:
        raise ValueError(f"SciFact archive MD5 mismatch: expected {SOURCE_MD5}, got {actual_md5}")
    extracted = raw / "extracted"
    if not extracted.exists():
        safe_extract(archive, extracted)
    dataset = locate_dataset(extracted)
    qrels_path = dataset / "qrels" / "test.tsv"
    corpus = load_jsonl(dataset / "corpus.jsonl")
    queries = load_jsonl(dataset / "queries.jsonl")
    qrels = read_qrels(qrels_path)
    judged_query_ids = {row["query-id"] for row in qrels if row["score"] > 0}
    query_ids = ranked(judged_query_ids, args.seed)[: args.queries]
    selected_queries = set(query_ids)
    selected_qrels = [row for row in qrels if row["query-id"] in selected_queries]
    relevant_ids = {row["corpus-id"] for row in selected_qrels if row["score"] > 0}
    if len(relevant_ids) > args.documents:
        raise ValueError(f"{len(relevant_ids)} relevant documents exceed requested corpus size")
    corpus_by_id = {str(row["_id"]): row for row in corpus}
    missing = relevant_ids - corpus_by_id.keys()
    if missing:
        raise ValueError(f"qrels reference missing corpus IDs: {sorted(missing)[:5]}")
    distractors = ranked(corpus_by_id.keys() - relevant_ids, args.seed + ":corpus")
    document_ids = relevant_ids | set(distractors[: args.documents - len(relevant_ids)])
    subset = output / f"h6-{args.documents}d-{args.queries}q-v1"
    if subset.exists():
        shutil.rmtree(subset)
    subset.mkdir(parents=True)
    selected_corpus = [corpus_by_id[key] for key in sorted(document_ids)]
    queries_by_id = {str(row["_id"]): row for row in queries}
    selected_query_rows = [queries_by_id[key] for key in sorted(selected_queries)]
    selected_qrels = [row for row in selected_qrels if row["corpus-id"] in document_ids]
    write_jsonl(subset / "corpus.jsonl", selected_corpus)
    write_jsonl(subset / "queries.jsonl", selected_query_rows)
    write_qrels(subset / "qrels.tsv", selected_qrels)
    manifest = {
        "schema": "zlf-scifact-subset-v1",
        "source_url": SOURCE_URL,
        "source_archive_md5": actual_md5,
        "source_archive_sha256": digest(archive),
        "seed": args.seed,
        "selection": "all positive qrel documents plus SHA-256-ranked distractors",
        "query_count": len(selected_query_rows),
        "document_count": len(selected_corpus),
        "positive_qrel_count": sum(row["score"] > 0 for row in selected_qrels),
        "files": {
            name: digest(subset / name)
            for name in ["corpus.jsonl", "queries.jsonl", "qrels.tsv"]
        },
    }
    (subset / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(subset)
    print(json.dumps(manifest, indent=2, sort_keys=True))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("data/benchmarks/scifact"))
    parser.add_argument("--documents", type=int, default=1000)
    parser.add_argument("--queries", type=int, default=100)
    parser.add_argument("--seed", default="zlf-h6-scifact-v1")
    args = parser.parse_args()
    if args.documents <= 0 or args.queries <= 0:
        parser.error("documents and queries must be positive")
    prepare(args)


if __name__ == "__main__":
    main()
