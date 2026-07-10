#!/usr/bin/env python3
"""Independent taxonomy-tree oracle for lineage/LCA/tree-edge distance."""

import argparse
import json
from pathlib import Path


def rows(path):
    with path.open(encoding="utf-8", errors="replace") as source:
        for line in source:
            text = line.rstrip("\n")
            if text.endswith("\t|"):
                text = text[:-2]
            yield [field.strip() for field in text.split("\t|\t")]


def parent_map(path, limit):
    parents = {}
    selected = set()
    pending = []
    for index, row in enumerate(rows(path)):
        if limit is not None and index >= limit:
            break
        selected.add(row[0])
        if row[0] != row[1]:
            pending.append((row[0], row[1]))
    for child, parent in pending:
        if parent in selected:
            parents[child] = parent
    return parents


def distances(source, parents):
    result = {source: 0}
    current = source
    seen = {source}
    while current in parents:
        current = parents[current]
        if current in seen:
            raise ValueError(f"cycle encountered at tax_{current}")
        result[current] = len(result)
        seen.add(current)
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("taxdump", type=Path)
    parser.add_argument("left")
    parser.add_argument("right")
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    parents = parent_map(args.taxdump / "nodes.dmp", args.limit)
    left = distances(args.left, parents)
    right = distances(args.right, parents)
    common = set(left) & set(right)
    lca = min(common, key=lambda taxon: left[taxon] + right[taxon]) if common else None
    children = {}
    for child, parent in parents.items():
        children.setdefault(parent, []).append(child)
    descendants = []
    stack = list(children.get(args.right, []))
    while stack:
        child = stack.pop()
        descendants.append(child)
        stack.extend(children.get(child, []))
    descendants.sort()
    print(
        json.dumps(
            {
                "left": f"tax_{args.left}",
                "right": f"tax_{args.right}",
                "left_ancestors": [f"tax_{taxon}" for taxon in left if taxon != args.left],
                "descendants_of_right": [f"tax_{taxon}" for taxon in descendants],
                "lca": f"tax_{lca}" if lca else None,
                "distance": left[lca] + right[lca] if lca else None,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
