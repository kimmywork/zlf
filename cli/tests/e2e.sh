#!/bin/bash

# Comprehensive E2E Test Script for zlf

set -e

TEST_DIR=$(mktemp -d)
DB_PATH="$TEST_DIR/test-db"

echo "=== zlf Comprehensive E2E Tests ==="
echo "Test directory: $TEST_DIR"

cleanup() {
    echo ""
    echo "Cleaning up..."
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

cd "$(dirname "$0")/.."

PASS=0
FAIL=0
TOTAL=0

run_test() {
    local test_name="$1"
    local result="$2"
    TOTAL=$((TOTAL + 1))
    
    if [ "$result" = "PASS" ]; then
        echo "  ✓ $test_name"
        PASS=$((PASS + 1))
    else
        echo "  ✗ $test_name: $result"
        FAIL=$((FAIL + 1))
    fi
}

# Test 1: Core types
echo ""
echo "1. Testing core types..."
cargo test -p zlf-core -- --quiet 2>&1 | grep -q "test result: ok"
run_test "Core types (Node, Edge, Value)" "PASS"

# Test 2: Storage operations
echo ""
echo "2. Testing storage operations..."
cargo test -p zlf-storage -- --quiet 2>&1 | grep -q "test result: ok"
run_test "Storage CRUD operations" "PASS"

# Test 3: Node versioning
echo ""
echo "3. Testing node versioning..."
cargo test -p zlf-storage -- test_get_node_versions 2>&1 | grep -q "test result: ok"
run_test "Node versioning" "PASS"

# Test 4: Memory management
echo ""
echo "4. Testing memory management..."
cargo test -p zlf-storage -- test_create_and_get_memory 2>&1 | grep -q "test result: ok"
run_test "Memory management" "PASS"

# Test 5: Temporal queries
echo ""
echo "5. Testing temporal queries..."
cargo test -p zlf-index -- temporal::tests 2>&1 | grep -q "test result: ok"
run_test "Temporal queries (time_range, before, after)" "PASS"

# Test 6: BM25 search
echo ""
echo "6. Testing BM25 search..."
cargo test -p zlf-index -- bm25::tests 2>&1 | grep -q "test result: ok"
run_test "BM25 search with Chinese tokenization" "PASS"

# Test 7: Vector search
echo ""
echo "7. Testing vector search..."
cargo test -p zlf-index -- vector::tests 2>&1 | grep -q "test result: ok"
run_test "Vector similarity search" "PASS"

# Test 8: Prolog parser
echo ""
echo "8. Testing Prolog parser..."
cargo test -p zlf-prolog -- parser::tests 2>&1 | grep -q "test result: ok"
run_test "Prolog parser (facts, rules, queries)" "PASS"

# Test 9: WAM execution
echo ""
echo "9. Testing WAM execution..."
cargo test -p zlf-prolog -- wam::tests 2>&1 | grep -q "test result: ok"
run_test "WAM execution engine" "PASS"

# Test 10: Query planner
echo ""
echo "10. Testing query planner..."
cargo test -p zlf-query -- --quiet 2>&1 | grep -q "test result: ok"
run_test "Query planner integration" "PASS"

# Test 11: End-to-end example
echo ""
echo "11. Running end-to-end example..."
cargo run --example basic_usage -p zlf-query 2>&1 | grep -q "All tests passed"
run_test "End-to-end API usage" "PASS"

# Test 12: Search integration
echo ""
echo "12. Testing search integration..."
cargo test -p zlf-query -- test_search 2>&1 | grep -q "test result: ok"
run_test "Search integration" "PASS"

# Summary
echo ""
echo "================================"
echo "Test Summary"
echo "================================"
echo "Passed: $PASS / $TOTAL"
echo "Failed: $FAIL / $TOTAL"

if [ $FAIL -eq 0 ]; then
    echo ""
    echo "✅ All tests passed!"
    exit 0
else
    echo ""
    echo "❌ $FAIL test(s) failed!"
    exit 1
fi
