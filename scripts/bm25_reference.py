#!/usr/bin/env python3
"""Small dependency-free BM25 oracle for fixtures and benchmark adapters."""

from __future__ import annotations

import math
from collections import Counter
from dataclasses import dataclass


@dataclass(frozen=True)
class Hit:
    document_id: str
    score: float


def rank(query: list[str], documents: dict[str, list[str]], k1: float = 1.2, b: float = 0.75) -> list[Hit]:
    lengths = {doc_id: len(tokens) for doc_id, tokens in documents.items()}
    average = sum(lengths.values()) / len(lengths) if lengths else 0.0
    frequencies = {doc_id: Counter(tokens) for doc_id, tokens in documents.items()}
    document_frequency = {
        term: sum(term in counts for counts in frequencies.values()) for term in set(query)
    }
    hits: list[Hit] = []
    for doc_id, counts in frequencies.items():
        score = sum(
            term_score(
                counts[term], document_frequency[term], len(documents), lengths[doc_id], average, k1, b
            )
            for term in set(query)
        )
        if score > 0.0:
            hits.append(Hit(doc_id, score))
    return sorted(hits, key=lambda hit: (-hit.score, hit.document_id))


def term_score(tf: int, df: int, count: int, length: int, average: float, k1: float, b: float) -> float:
    if not tf or not df or not count or average <= 0.0:
        return 0.0
    inverse_document_frequency = math.log(1.0 + (count - df + 0.5) / (df + 0.5))
    normalization = 1.0 - b + b * length / average
    return inverse_document_frequency * tf * (k1 + 1.0) / (tf + k1 * normalization)


if __name__ == "__main__":
    fixture = {"a": "rust rust graph".split(), "b": "rust database storage".split()}
    for result in rank(["rust"], fixture):
        print(f"{result.document_id}\t{result.score:.9f}")
