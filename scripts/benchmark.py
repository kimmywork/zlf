#!/usr/bin/env python3
"""Benchmark suite for zlf."""

import json
import subprocess
import time
import statistics
from pathlib import Path

DB_PATH = "./wiki-benchmark-db"
ZLF_BINARY = "./target/debug/zlf"

def run_zlf(request: dict) -> dict:
    """Execute a zlf command and return the response."""
    cmd = json.dumps(request)
    start = time.time()
    result = subprocess.run(
        [ZLF_BINARY],
        input=cmd,
        capture_output=True,
        text=True,
        timeout=60
    )
    elapsed = time.time() - start
    return json.loads(result.stdout.strip()), elapsed

def benchmark_init():
    """Benchmark database initialization."""
    print("=== Benchmark: Database Init ===")
    times = []
    for i in range(5):
        db_path = f"{DB_PATH}-{i}"
        _, elapsed = run_zlf({"command": "init", "path": db_path})
        times.append(elapsed)
    
    avg = statistics.mean(times)
    print(f"  Average: {avg:.3f}s (min: {min(times):.3f}s, max: {max(times):.3f}s)")
    return avg

def benchmark_add_node():
    """Benchmark adding nodes."""
    print("\n=== Benchmark: Add Node ===")
    db_path = f"{DB_PATH}-add"
    run_zlf({"command": "init", "path": db_path})
    
    times = []
    for i in range(100):
        _, elapsed = run_zlf({
            "command": "add_node",
            "path": db_path,
            "labels": ["person"],
            "properties": {"name": f"User{i}", "age": i % 100}
        })
        times.append(elapsed)
    
    avg = statistics.mean(times)
    print(f"  100 nodes: avg {avg*1000:.1f}ms/node")
    print(f"  Total: {sum(times):.3f}s")
    return avg

def benchmark_add_edge():
    """Benchmark adding edges."""
    print("\n=== Benchmark: Add Edge ===")
    db_path = f"{DB_PATH}-edge"
    run_zlf({"command": "init", "path": db_path})
    
    # Create nodes first
    node_ids = []
    for i in range(50):
        resp, _ = run_zlf({
            "command": "add_node",
            "path": db_path,
            "labels": ["person"],
            "properties": {"name": f"User{i}"}
        })
        node_ids.append(resp["data"]["id"])
    
    # Add edges
    times = []
    for i in range(len(node_ids) - 1):
        _, elapsed = run_zlf({
            "command": "add_edge",
            "path": db_path,
            "edge_type": "knows",
            "source": node_ids[i],
            "target": node_ids[i + 1],
            "properties": {}
        })
        times.append(elapsed)
    
    avg = statistics.mean(times)
    print(f"  49 edges: avg {avg*1000:.1f}ms/edge")
    print(f"  Total: {sum(times):.3f}s")
    return avg

def benchmark_query():
    """Benchmark queries."""
    print("\n=== Benchmark: Query ===")
    db_path = f"{DB_PATH}-query"
    run_zlf({"command": "init", "path": db_path})
    
    # Add nodes
    for i in range(200):
        run_zlf({
            "command": "add_node",
            "path": db_path,
            "labels": ["person"] if i % 2 == 0 else ["company"],
            "properties": {"name": f"Entity{i}"}
        })
    
    # Benchmark queries
    queries = [
        ("?node(person, X, Props).", "node by label"),
        ("?node(X, Y, Z).", "all nodes"),
    ]
    
    for query, desc in queries:
        times = []
        for _ in range(20):
            _, elapsed = run_zlf({
                "command": "query",
                "path": db_path,
                "query": query
            })
            times.append(elapsed)
        
        avg = statistics.mean(times)
        print(f"  {desc}: avg {avg*1000:.1f}ms (p50: {statistics.median(times)*1000:.1f}ms)")

def benchmark_search():
    """Benchmark BM25 search."""
    print("\n=== Benchmark: BM25 Search ===")
    db_path = f"{DB_PATH}-search"
    run_zlf({"command": "init", "path": db_path})
    
    # Add nodes with text
    for i in range(100):
        resp, _ = run_zlf({
            "command": "add_node",
            "path": db_path,
            "labels": ["document"],
            "properties": {"title": f"Document {i}", "content": f"This is document number {i} about topic {i % 10}"}
        })
        # Index text
        run_zlf({
            "command": "index_text",
            "path": db_path,
            "node_id": resp["data"]["id"],
            "text": f"Document {i} topic {i % 10}"
        })
    
    # Benchmark search
    queries = ["document", "topic", "number", "about"]
    for query in queries:
        times = []
        for _ in range(10):
            _, elapsed = run_zlf({
                "command": "search",
                "path": db_path,
                "query": query
            })
            times.append(elapsed)
        
        avg = statistics.mean(times)
        print(f"  Search '{query}': avg {avg*1000:.1f}ms")

def benchmark_embedding():
    """Benchmark embedding generation."""
    print("\n=== Benchmark: Embedding ===")
    
    texts = [
        "Hello world",
        "This is a longer text for embedding benchmark",
        "机器学习是人工智能的一个分支",
        "A" * 500,  # Long text
    ]
    
    for text in texts:
        times = []
        for _ in range(5):
            _, elapsed = run_zlf({
                "command": "embed",
                "text": text,
                "config": {
                    "provider": {"type": "ollama"},
                    "api_endpoint": "http://localhost:11434",
                    "model": "bge-m3:latest",
                    "dimension": 1024
                }
            })
            times.append(elapsed)
        
        avg = statistics.mean(times)
        print(f"  Embed ({len(text)} chars): avg {avg*1000:.1f}ms")

def main():
    print("=== zlf Benchmark Suite ===\n")
    
    # Clean up
    import shutil
    for p in Path(".").glob(f"{DB_PATH}*"):
        if p.is_dir():
            shutil.rmtree(p)
    
    # Run benchmarks
    benchmark_init()
    benchmark_add_node()
    benchmark_add_edge()
    benchmark_query()
    benchmark_search()
    benchmark_embedding()
    
    print("\n=== Benchmark Complete ===")

if __name__ == "__main__":
    main()
