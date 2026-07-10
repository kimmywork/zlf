#!/usr/bin/env python3
"""Stream an NCBI taxdump into deterministic, sharded ground Prolog facts."""

import argparse
import hashlib
import itertools
import json
from pathlib import Path


def dmp_rows(path):
    with path.open(encoding="utf-8", errors="replace") as source:
        for line in source:
            text = line.rstrip("\n")
            if text.endswith("\t|"):
                text = text[:-2]
            yield [field.strip() for field in text.split("\t|\t")]


def name_groups(path):
    rows = dmp_rows(path)
    for tax_id, group in itertools.groupby(rows, key=lambda row: int(row[0])):
        yield tax_id, list(group)


def quote(value):
    escaped = (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\t", "\\t")
        .replace("\r", "\\r")
    )
    return f'"{escaped}"'


def object_term(entries):
    return "{" + ", ".join(f"{key}: {value}" for key, value in entries) + "}"


def name_term(row):
    return object_term(
        [
            ("text", quote(row[1])),
            ("unique_name", quote(row[2])),
            ("name_class", quote(row[3])),
        ]
    )


def taxon_fact(row, names):
    scientific = next((name[1] for name in names if name[3] == "scientific name"), "")
    properties = [
        ("tax_id", row[0]),
        ("scientific_name", quote(scientific)),
        ("rank", quote(row[2])),
        ("division_id", row[4]),
        ("genetic_code_id", row[6]),
        ("mitochondrial_genetic_code_id", row[8]),
        ("genbank_hidden", row[10]),
        ("hidden_subtree_root", row[11]),
        ("names", "[" + ", ".join(name_term(name) for name in names) + "]"),
    ]
    return f"node(tax_{row[0]}, [taxon], {object_term(properties)})."


class ShardWriter:
    def __init__(self, output, prefix, shard_size):
        self.output = output
        self.prefix = prefix
        self.shard_size = shard_size
        self.shard = -1
        self.count = 0
        self.handle = None
        self.paths = []

    def write(self, fact):
        if self.handle is None or self.count % self.shard_size == 0:
            if self.handle:
                self.handle.close()
            self.shard += 1
            path = self.output / f"{self.prefix}-{self.shard:05d}.pl"
            self.handle = path.open("w", encoding="utf-8")
            self.paths.append(path)
        self.handle.write(fact)
        self.handle.write("\n")
        self.count += 1

    def close(self):
        if self.handle:
            self.handle.close()


def convert_nodes(taxdump, output, shard_size, limit):
    taxa = ShardWriter(output, "10-taxa", shard_size)
    parents = ShardWriter(output, "30-parent-edges", shard_size)
    groups = iter(name_groups(taxdump / "names.dmp"))
    current = next(groups, None)
    selected = set() if limit is not None else None
    pending_edges = []
    for index, row in enumerate(dmp_rows(taxdump / "nodes.dmp")):
        if limit is not None and index >= limit:
            break
        tax_id = int(row[0])
        while current and current[0] < tax_id:
            current = next(groups, None)
        names = current[1] if current and current[0] == tax_id else []
        taxa.write(taxon_fact(row, names))
        if selected is not None:
            selected.add(row[0])
        if row[0] != row[1]:
            if selected is None:
                parents.write(f"taxonomy_parent(tax_{row[0]}, tax_{row[1]}).")
            else:
                pending_edges.append((row[0], row[1]))
    if selected is not None:
        for child, parent in pending_edges:
            if parent in selected:
                parents.write(f"taxonomy_parent(tax_{child}, tax_{parent}).")
    taxa.close()
    parents.close()
    return taxa, parents


def convert_simple(taxdump, output, shard_size):
    merged_nodes = ShardWriter(output, "40-merged-nodes", shard_size)
    merged_edges = ShardWriter(output, "41-merged-edges", shard_size)
    for row in dmp_rows(taxdump / "merged.dmp"):
        merged_nodes.write(f"node(tax_{row[0]}, [merged_taxon], {{tax_id: {row[0]}}}).")
        merged_edges.write(f"merged_into(tax_{row[0]}, tax_{row[1]}).")
    deleted = ShardWriter(output, "50-deleted", shard_size)
    for row in dmp_rows(taxdump / "delnodes.dmp"):
        deleted.write(f"node(tax_{row[0]}, [deleted_taxon], {{tax_id: {row[0]}}}).")
    for writer in (merged_nodes, merged_edges, deleted):
        writer.close()
    return [merged_nodes, merged_edges, deleted]


def checksum(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("taxdump", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--shard-size", type=int, default=50_000)
    parser.add_argument("--skip-history", action="store_true")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    writers = list(convert_nodes(args.taxdump, args.output, args.shard_size, args.limit))
    if not args.skip_history and args.limit is None:
        writers.extend(convert_simple(args.taxdump, args.output, args.shard_size))
    shards = [path for writer in writers for path in writer.paths]
    manifest = {
        "format": "zlf-ncbi-taxonomy-facts-v1",
        "source": {
            name: checksum(args.taxdump / name)
            for name in ("nodes.dmp", "names.dmp", "merged.dmp", "delnodes.dmp")
        },
        "limit": args.limit,
        "shards": [
            {"path": path.name, "facts": sum(1 for _ in path.open()), "sha256": checksum(path)}
            for path in shards
        ],
    }
    (args.output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"facts": sum(item["facts"] for item in manifest["shards"]), "shards": len(shards)}))


if __name__ == "__main__":
    main()
