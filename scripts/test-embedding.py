#!/usr/bin/env python3
"""Test embedding providers."""

import json
import subprocess
import time

ZLF_BINARY = "./target/debug/zlf"

def run_zlf(request: dict) -> dict:
    """Execute a zlf command and return the response."""
    cmd = json.dumps(request)
    result = subprocess.run(
        [ZLF_BINARY],
        input=cmd,
        capture_output=True,
        text=True,
        timeout=60
    )
    return json.loads(result.stdout.strip())

def test_ollama_embedding():
    """Test Ollama embedding provider."""
    print("=== Test 1: Ollama Embedding ===")
    start = time.time()
    response = run_zlf({
        "command": "embed",
        "text": "Hello world, this is a test of the embedding system",
        "config": {
            "provider": {"type": "ollama"},
            "api_endpoint": "http://localhost:11434",
            "model": "bge-m3:latest",
            "dimension": 1024
        }
    })
    elapsed = time.time() - start
    
    assert response["type"] == "success", f"Failed: {response}"
    embedding = response["data"]["embedding"]
    print(f"  Dimension: {len(embedding)}")
    print(f"  Time: {elapsed:.3f}s")
    print(f"  First 5 values: {embedding[:5]}")
    print("  ✓ PASSED\n")

def test_openai_compatible_embedding():
    """Test OpenAI-compatible embedding via Ollama."""
    print("=== Test 2: OpenAI-compatible Embedding (via Ollama) ===")
    start = time.time()
    response = run_zlf({
        "command": "embed",
        "text": "Hello world, this is a test of the embedding system",
        "config": {
            "provider": {"type": "openai"},
            "api_endpoint": "http://localhost:11434",
            "model": "bge-m3:latest",
            "dimension": 1024
        }
    })
    elapsed = time.time() - start
    
    assert response["type"] == "success", f"Failed: {response}"
    embedding = response["data"]["embedding"]
    print(f"  Dimension: {len(embedding)}")
    print(f"  Time: {elapsed:.3f}s")
    print(f"  First 5 values: {embedding[:5]}")
    print("  ✓ PASSED\n")

def test_chinese_embedding():
    """Test Chinese text embedding."""
    print("=== Test 3: Chinese Text Embedding ===")
    start = time.time()
    response = run_zlf({
        "command": "embed",
        "text": "这是一个中文文本嵌入测试",
        "config": {
            "provider": {"type": "ollama"},
            "api_endpoint": "http://localhost:11434",
            "model": "bge-m3:latest",
            "dimension": 1024
        }
    })
    elapsed = time.time() - start
    
    assert response["type"] == "success", f"Failed: {response}"
    embedding = response["data"]["embedding"]
    print(f"  Dimension: {len(embedding)}")
    print(f"  Time: {elapsed:.3f}s")
    print("  ✓ PASSED\n")

def test_batch_embedding():
    """Test batch embedding (multiple texts)."""
    print("=== Test 4: Batch Embedding ===")
    texts = [
        "First text for embedding",
        "Second text for embedding",
        "Third text for embedding"
    ]
    
    embeddings = []
    start = time.time()
    for text in texts:
        response = run_zlf({
            "command": "embed",
            "text": text,
            "config": {
                "provider": {"type": "ollama"},
                "api_endpoint": "http://localhost:11434",
                "model": "bge-m3:latest",
                "dimension": 1024
            }
        })
        assert response["type"] == "success", f"Failed: {response}"
        embeddings.append(response["data"]["embedding"])
    elapsed = time.time() - start
    
    print(f"  Embedded {len(texts)} texts in {elapsed:.3f}s")
    print(f"  Average time per text: {elapsed/len(texts):.3f}s")
    print("  ✓ PASSED\n")

def main():
    print("=== Embedding Provider Test Suite ===\n")
    
    tests = [
        test_ollama_embedding,
        test_openai_compatible_embedding,
        test_chinese_embedding,
        test_batch_embedding,
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
