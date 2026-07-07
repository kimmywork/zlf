#!/usr/bin/env python3
"""Import wiki markdown files into zlf database."""

import os
import json
import subprocess
import sys
from pathlib import Path

WIKI_DIR = Path.home() / "workspace/docs/wiki/content"
DB_PATH = "./wiki-db"
ZLF_BINARY = "./target/release/zlf"

def run_zlf(request: dict) -> dict:
    """Execute a zlf command and return the response."""
    cmd = json.dumps(request)
    result = subprocess.run(
        [ZLF_BINARY],
        input=cmd,
        capture_output=True,
        text=True,
        timeout=30
    )
    return json.loads(result.stdout.strip())

def import_file(file_path: Path) -> bool:
    """Import a single markdown file."""
    try:
        # Read file content
        content = file_path.read_text(encoding='utf-8', errors='ignore')
        
        # Extract filename
        filename = file_path.stem
        
        # Extract directory structure for labels
        rel_path = file_path.relative_to(WIKI_DIR)
        dir_parts = list(rel_path.parent.parts)
        
        # Create labels
        labels = ["article"] + dir_parts if dir_parts else ["article"]
        
        # Truncate content for properties
        content_truncated = content[:2000].replace('"', '\\"').replace('\n', ' ')
        
        # Create request
        request = {
            "command": "add_node",
            "path": DB_PATH,
            "labels": labels,
            "properties": {
                "title": filename,
                "path": str(rel_path),
                "content": content_truncated
            }
        }
        
        response = run_zlf(request)
        
        # If successful, also index the text content for BM25 search
        if response.get("type") == "success":
            node_id = response["data"]["id"]
            # Index title and content for search
            index_request = {
                "command": "index_text",
                "path": DB_PATH,
                "node_id": node_id,
                "text": f"{filename} {content[:1000]}"
            }
            run_zlf(index_request)
        
        return response.get("type") == "success"
        
    except Exception as e:
        print(f"  Error importing {file_path.name}: {e}", file=sys.stderr)
        return False

def main():
    print("=== Importing Wiki Content into zlf ===")
    print(f"Source: {WIKI_DIR}")
    print(f"Database: {DB_PATH}")
    
    # Initialize database
    print("\n1. Initializing database...")
    response = run_zlf({"command": "init", "path": DB_PATH})
    if response.get("type") != "success":
        print(f"Failed to initialize database: {response}")
        return 1
    
    # Find all markdown files
    md_files = list(WIKI_DIR.rglob("*.md"))
    total = len(md_files)
    print(f"\nFound {total} markdown files")
    
    # Import files
    print("\n2. Importing files...")
    success_count = 0
    error_count = 0
    
    for i, file_path in enumerate(md_files, 1):
        if import_file(file_path):
            success_count += 1
        else:
            error_count += 1
        
        if i % 50 == 0:
            print(f"  Progress: {i}/{total} ({success_count} success, {error_count} errors)")
    
    print(f"\n3. Import complete!")
    print(f"  Total: {total}")
    print(f"  Success: {success_count}")
    print(f"  Errors: {error_count}")
    
    # Verify import
    print("\n4. Verifying import...")
    response = run_zlf({
        "command": "query",
        "path": DB_PATH,
        "query": "?node(article, X, Props)."
    })
    
    if response.get("type") == "success":
        data = response.get("data", [])
        print(f"  Query returned {len(data)} results")
        if data:
            print(f"  First result: {json.dumps(data[0], indent=2)[:200]}...")
    else:
        print(f"  Query failed: {response}")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
