#!/usr/bin/env python3
"""Test cases for zlf wiki import."""

import json
import subprocess
import time
from pathlib import Path

DB_PATH = "./wiki-db"
ZLF_BINARY = "./target/release/zlf"

def run_zlf(request: dict) -> dict:
    """Execute a zlf command and return the response."""
    cmd = json.dumps(request)
    start = time.time()
    result = subprocess.run(
        [ZLF_BINARY],
        input=cmd,
        capture_output=True,
        text=True,
        timeout=30
    )
    elapsed = time.time() - start
    response = json.loads(result.stdout.strip())
    return response, elapsed

def test_basic_query():
    """Test basic node query."""
    print("=== Test 1: Basic Node Query ===")
    response, elapsed = run_zlf({
        "command": "query",
        "path": DB_PATH,
        "query": "?node(article, X, Props)."
    })
    assert response["type"] == "success", f"Query failed: {response}"
    data = response["data"]
    print(f"  Found {len(data)} nodes in {elapsed:.3f}s")
    assert len(data) == 517, f"Expected 517 nodes, got {len(data)}"
    print("  ✓ PASSED\n")

def test_label_filter():
    """Test label filtering."""
    print("=== Test 2: Label Filtering ===")
    response, elapsed = run_zlf({
        "command": "query",
        "path": DB_PATH,
        "query": "?node(collect, X, Props)."
    })
    assert response["type"] == "success", f"Query failed: {response}"
    data = response["data"]
    print(f"  Found {len(data)} nodes with 'collect' label in {elapsed:.3f}s")
    assert len(data) == 334, f"Expected 334 nodes, got {len(data)}"
    print("  ✓ PASSED\n")

def test_get_node_by_id():
    """Test getting a specific node by ID."""
    print("=== Test 3: Get Node by ID ===")
    # First get a node ID from the query
    response, _ = run_zlf({
        "command": "query",
        "path": DB_PATH,
        "query": "?node(article, X, Props)."
    })
    node_id = response["data"][0]["id"]
    
    # Get the node by ID
    response, elapsed = run_zlf({
        "command": "get_node",
        "path": DB_PATH,
        "id": node_id
    })
    assert response["type"] == "success", f"Get node failed: {response}"
    node = response["data"]
    print(f"  Retrieved node {node_id} in {elapsed:.3f}s")
    assert node["id"] == node_id, "Node ID mismatch"
    print("  ✓ PASSED\n")

def test_search():
    """Test BM25 search."""
    print("=== Test 4: BM25 Search ===")
    response, elapsed = run_zlf({
        "command": "search",
        "path": DB_PATH,
        "query": "python"
    })
    assert response["type"] == "success", f"Search failed: {response}"
    data = response["data"]
    print(f"  Found {len(data)} results for 'python' in {elapsed:.3f}s")
    if data:
        print(f"  Top result: {data[0]['node_id']} (score: {data[0]['score']:.2f})")
    print("  ✓ PASSED\n")

def test_chinese_search():
    """Test Chinese text search."""
    print("=== Test 5: Chinese Text Search ===")
    response, elapsed = run_zlf({
        "command": "search",
        "path": DB_PATH,
        "query": "算法"
    })
    assert response["type"] == "success", f"Search failed: {response}"
    data = response["data"]
    print(f"  Found {len(data)} results for '算法' in {elapsed:.3f}s")
    print("  ✓ PASSED\n")

def test_performance():
    """Test query performance."""
    print("=== Test 6: Performance Test ===")
    queries = [
        ("?node(article, X, Props).", "all articles"),
        ("?node(collect, X, Props).", "collect nodes"),
    ]
    
    for query, desc in queries:
        # Run 10 queries and measure average time
        times = []
        for _ in range(10):
            _, elapsed = run_zlf({
                "command": "query",
                "path": DB_PATH,
                "query": query
            })
            times.append(elapsed)
        
        avg_time = sum(times) / len(times)
        print(f"  {desc}: avg {avg_time:.3f}s (min: {min(times):.3f}s, max: {max(times):.3f}s)")
    
    print("  ✓ PASSED\n")

def test_add_and_query():
    """Test adding a node and querying it."""
    print("=== Test 7: Add and Query Node ===")
    # Add a node
    response, elapsed = run_zlf({
        "command": "add_node",
        "path": DB_PATH,
        "labels": ["test", "article"],
        "properties": {
            "title": "test-article",
            "content": "This is a test article"
        }
    })
    assert response["type"] == "success", f"Add node failed: {response}"
    node_id = response["data"]["id"]
    print(f"  Added node {node_id} in {elapsed:.3f}s")
    
    # Query for the node
    response, elapsed = run_zlf({
        "command": "query",
        "path": DB_PATH,
        "query": "?node(test, X, Props)."
    })
    assert response["type"] == "success", f"Query failed: {response}"
    data = response["data"]
    assert len(data) > 0, "Node not found after adding"
    print(f"  Queried node in {elapsed:.3f}s")
    print("  ✓ PASSED\n")

def main():
    print("=== zlf Wiki Import Test Suite ===\n")
    
    tests = [
        test_basic_query,
        test_label_filter,
        test_get_node_by_id,
        test_search,
        test_chinese_search,
        test_performance,
        test_add_and_query,
    ]
    
    passed = 0
    failed = 0
    
    for test in tests:
        try:
            test()
            passed += 1
        except Exception as e:
            print(f"  ✗ FAILED: {e}\n")
            failed += 1
    
    print("=== Test Summary ===")
    print(f"Passed: {passed}/{len(tests)}")
    print(f"Failed: {failed}/{len(tests)}")
    
    return 0 if failed == 0 else 1

if __name__ == "__main__":
    exit(main())
