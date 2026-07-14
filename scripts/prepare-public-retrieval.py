#!/usr/bin/env python3
"""Prepare deterministic FiQA and bounded MIRACL en/zh retrieval datasets."""

import argparse
import csv
import gzip
import hashlib
import json
import shutil
from pathlib import Path

import pyarrow.parquet as pq


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def ranked(values, seed):
    return sorted(values, key=lambda value: (hashlib.sha256(f"{seed}\0{value}".encode()).digest(), value))


def write_jsonl(path, rows):
    with path.open("w", encoding="utf-8") as stream:
        for row in rows:
            stream.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def write_qrels(path, rows):
    with path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
        writer.writerow(["query-id", "corpus-id", "score"])
        writer.writerows(rows)


def read_beir_qrels(path):
    with path.open(encoding="utf-8", newline="") as stream:
        return [(row["query-id"], row["corpus-id"], int(row["score"])) for row in csv.DictReader(stream, delimiter="\t")]


def read_miracl_qrels(path):
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        query, _, document, score = line.split("\t")
        rows.append((query, document, int(score)))
    return rows


def prepare_subset(output, corpus, queries, qrels, query_limit, document_limit, seed, metadata, hard_negatives=False):
    positives = {}
    for query, document, score in qrels:
        if score > 0:
            positives.setdefault(query, set()).add(document)
    corpus_ids = set(corpus)
    eligible = [query for query, documents in positives.items() if query in queries and documents <= corpus_ids]
    selected_queries = ranked(eligible, seed + ":queries")[:query_limit]
    selected = set(selected_queries)
    required = set().union(*(positives[query] for query in selected_queries)) if selected_queries else set()
    if hard_negatives:
        required.update(document for query, document, _ in qrels if query in selected and document in corpus_ids)
    if len(required) > document_limit:
        raise ValueError(f"{len(required)} judged documents exceed document limit {document_limit}")
    distractors = ranked(corpus_ids - required, seed + ":documents")
    document_ids = required | set(distractors[: document_limit - len(required)])
    subset_qrels = sorted((query, document, score) for query, document, score in qrels if query in selected and document in document_ids)
    if any(not positives[query] <= document_ids for query in selected_queries):
        raise ValueError("positive judgment was lost during preparation")
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    write_jsonl(output / "corpus.jsonl", [corpus[key] for key in sorted(document_ids)])
    write_jsonl(output / "queries.jsonl", [queries[key] for key in sorted(selected)])
    write_qrels(output / "qrels.tsv", subset_qrels)
    manifest = {
        "schema": "zlf-public-retrieval-subset-v1", "seed": seed,
        "selection": "all positive judgments, optional judged negatives, then SHA-256-ranked distractors",
        "document_count": len(document_ids), "query_count": len(selected),
        "positive_qrel_count": sum(score > 0 for _, _, score in subset_qrels),
        "files": {name: digest(output / name) for name in ("corpus.jsonl", "queries.jsonl", "qrels.tsv")},
        **metadata,
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(output)


def prepare_fiqa(root, output, documents, queries, seed):
    corpus_path = root / "fiqa/corpus/corpus-00000-of-00001.parquet"
    queries_path = root / "fiqa/queries/queries-00000-of-00001.parquet"
    qrels_path = root / "fiqa-qrels/test.tsv"
    corpus = {str(row["_id"]): {"_id": str(row["_id"]), "title": row["title"] or "", "text": row["text"]} for row in pq.read_table(corpus_path).to_pylist()}
    query_rows = {str(row["_id"]): {"_id": str(row["_id"]), "text": row["text"]} for row in pq.read_table(queries_path).to_pylist()}
    prepare_subset(output / f"fiqa-{documents}d-{queries}q-v1", corpus, query_rows, read_beir_qrels(qrels_path), queries, documents, seed,
        {"dataset":"fiqa", "split":"test", "license":{"status":"pending_upstream_review","reported":"CC-BY-SA-4.0"},
         "source_files":{str(path.relative_to(root)):digest(path) for path in (corpus_path, queries_path, qrels_path)}})


def prepare_miracl(root, output, language, documents, query_limit, seed):
    corpus_path = root / f"miracl-corpus/miracl-corpus-v1.0-{language}/docs-0.jsonl.gz"
    topics_path = root / f"miracl/miracl-v1.0-{language}/topics/topics.miracl-v1.0-{language}-dev.tsv"
    qrels_path = root / f"miracl/miracl-v1.0-{language}/qrels/qrels.miracl-v1.0-{language}-dev.tsv"
    corpus = {}
    with gzip.open(corpus_path, "rt", encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line); key = str(row["docid"])
            corpus[key] = {"_id": key, "title": row.get("title", ""), "text": row["text"], "language": language}
    query_rows = {}
    for line in topics_path.read_text(encoding="utf-8").splitlines():
        key, text = line.split("\t", 1); query_rows[key] = {"_id": key, "text": text, "language": language}
    qrels = read_miracl_qrels(qrels_path)
    positives = {}
    for query, document, score in qrels:
        if score > 0: positives.setdefault(query, set()).add(document)
    complete = sum(documents_set <= corpus.keys() for documents_set in positives.values())
    prepare_subset(output / f"miracl-{language}-shard0-{documents}d-v1", corpus, query_rows, qrels, query_limit, documents, seed + f":{language}",
        {"dataset":"miracl", "language":language, "split":"dev", "corpus_scope":"shard-0 judged pool",
         "eligible_complete_positive_queries":complete, "license":{"status":"pending_upstream_review","reported_annotations":"Apache-2.0","corpus_origin":"Wikipedia"},
         "source_files":{str(path.relative_to(root)):digest(path) for path in (corpus_path, topics_path, qrels_path)}}, hard_negatives=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--data-root", type=Path, default=Path("data"))
    parser.add_argument("--output", type=Path, default=Path("data/benchmarks/public-retrieval"))
    parser.add_argument("--documents", type=int, default=10_000)
    parser.add_argument("--queries", type=int, default=100)
    parser.add_argument("--seed", default="zlf-public-retrieval-v1")
    args = parser.parse_args()
    if args.documents <= 0 or args.documents > 100_000 or args.queries <= 0:
        parser.error("documents must be in 1..=100000 and queries must be positive")
    root = args.data_root.resolve(); output = args.output.resolve()
    prepare_fiqa(root, output, args.documents, args.queries, args.seed + ":fiqa")
    for language in ("en", "zh"):
        prepare_miracl(root, output, language, args.documents, args.queries, args.seed)


if __name__ == "__main__": main()
