#!/usr/bin/env python3
"""Create one immutable 100K x 1024 f32 dataset for vector-engine comparisons."""

import argparse
import hashlib
import json
import os
import shutil
from pathlib import Path

import numpy as np

SCHEMA = "zlf-vector-search-dataset-v1"
ALGORITHM = "splitmix64-high24-uniform-l2-v1"
MASK = np.uint64(0xFFFFFFFFFFFFFFFF)
GAMMA = np.uint64(0x9E3779B97F4A7C15)
MUL1 = np.uint64(0xBF58476D1CE4E5B9)
MUL2 = np.uint64(0x94D049BB133111EB)
QUERY_DOMAIN = np.uint64(0xD1B54A32D192ED03)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def splitmix64(values):
    values = (values + GAMMA) & MASK
    values = ((values ^ (values >> np.uint64(30))) * MUL1) & MASK
    values = ((values ^ (values >> np.uint64(27))) * MUL2) & MASK
    return values ^ (values >> np.uint64(31))


def vectors(start, count, dimension, seed, domain=0):
    offsets = np.arange(count * dimension, dtype=np.uint64).reshape(count, dimension)
    offsets += np.uint64(start * dimension)
    mixed = splitmix64(offsets ^ np.uint64(seed) ^ np.uint64(domain))
    values = ((mixed >> np.uint64(40)).astype(np.float32) / np.float32(8388608.0)) - np.float32(1.0)
    norms = np.sqrt(np.sum(values.astype(np.float64) ** 2, axis=1, keepdims=True))
    values = (values.astype(np.float64) / norms).astype("<f4")
    return values


def write_documents(path, count, dimension, seed, batch):
    with path.open("wb") as stream:
        for start in range(0, count, batch):
            size = min(batch, count - start)
            stream.write(vectors(start, size, dimension, seed).tobytes(order="C"))


def write_metadata(path, count, seed, batch):
    with path.open("wb") as stream:
        for start in range(0, count, batch):
            ids = np.arange(start, min(start + batch, count), dtype=np.uint64)
            groups = (splitmix64(ids ^ np.uint64(seed)) % np.uint64(1000)).astype("<u2")
            stream.write(groups.tobytes(order="C"))


def write_queries(path, documents_path, document_count, query_count, self_count, dimension, seed, batch):
    document_vectors = np.memmap(documents_path, mode="r", dtype="<f4", shape=(document_count, dimension))
    self_ids = np.array([(index * 997) % document_count for index in range(self_count)], dtype=np.uint32)
    with path.open("wb") as stream:
        stream.write(np.asarray(document_vectors[self_ids], dtype="<f4").tobytes(order="C"))
        independent = query_count - self_count
        for start in range(0, independent, batch):
            size = min(batch, independent - start)
            stream.write(vectors(start, size, dimension, seed, int(QUERY_DOMAIN)).tobytes(order="C"))
    return self_ids


def verify(output, manifest):
    expected_sizes = {
        "documents.f32le": manifest["document_count"] * manifest["dimension"] * 4,
        "queries.f32le": manifest["query_count"] * manifest["dimension"] * 4,
        "document-groups.u16le": manifest["document_count"] * 2,
        "self-query-document-ids.u32le": manifest["self_query_count"] * 4,
    }
    for name, checksum in manifest["files"].items():
        path = output / name
        if not path.is_file() or path.stat().st_size != expected_sizes[name]:
            raise ValueError(f"missing or invalid-sized dataset file: {name}")
        actual = sha256(path)
        if actual != checksum:
            raise ValueError(f"checksum mismatch for {name}: {actual}")


def prepare(args):
    output = args.output.resolve()
    manifest_path = output / "manifest.json"
    if manifest_path.exists() and not args.force:
        manifest = json.loads(manifest_path.read_text())
        verify(output, manifest)
        print(f"verified existing immutable dataset: {output}")
        return
    temporary = output.with_name(output.name + ".tmp")
    if temporary.exists():
        shutil.rmtree(temporary)
    temporary.mkdir(parents=True)
    documents = temporary / "documents.f32le"
    queries = temporary / "queries.f32le"
    groups = temporary / "document-groups.u16le"
    self_ids_path = temporary / "self-query-document-ids.u32le"
    write_documents(documents, args.documents, args.dimension, args.seed, args.batch_size)
    write_metadata(groups, args.documents, args.seed, args.batch_size)
    self_ids = write_queries(queries, documents, args.documents, args.queries, args.self_queries, args.dimension, args.seed, args.batch_size)
    self_ids.astype("<u4").tofile(self_ids_path)
    files = [documents, queries, groups, self_ids_path]
    manifest = {
        "schema": SCHEMA, "algorithm": ALGORITHM, "seed": args.seed,
        "document_count": args.documents, "query_count": args.queries,
        "self_query_count": args.self_queries, "dimension": args.dimension,
        "dtype": "float32-little-endian", "normalized": True, "metric": "cosine",
        "document_id_pattern": "doc-{index:06d}",
        "query_layout": "self-query copies followed by independent deterministic queries",
        "group_layout": "uint16 little-endian values in [0,1000)",
        "numpy_version": np.__version__,
        "files": {path.name: sha256(path) for path in files},
        "file_sizes": {path.name: path.stat().st_size for path in files},
    }
    (temporary / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    if output.exists():
        shutil.rmtree(output)
    os.replace(temporary, output)
    verify(output, manifest)
    print(output)
    print(json.dumps(manifest, indent=2, sort_keys=True))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("data/benchmarks/vector-search-100k-1024-v1"))
    parser.add_argument("--documents", type=int, default=100_000)
    parser.add_argument("--queries", type=int, default=1_000)
    parser.add_argument("--self-queries", type=int, default=100)
    parser.add_argument("--dimension", type=int, default=1024)
    parser.add_argument("--seed", type=int, default=20260714)
    parser.add_argument("--batch-size", type=int, default=512)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()
    if not 0 < args.self_queries <= args.queries or min(args.documents, args.dimension, args.batch_size) <= 0:
        parser.error("invalid dataset dimensions or query counts")
    prepare(args)


if __name__ == "__main__":
    main()
