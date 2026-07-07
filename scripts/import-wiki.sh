#!/bin/bash
# Import wiki markdown files into zlf database

set -e

WIKI_DIR="$HOME/workspace/docs/wiki/content"
DB_PATH="./wiki-db"

echo "=== Importing Wiki Content into zlf ==="
echo "Source: $WIKI_DIR"
echo "Database: $DB_PATH"

# Initialize database
echo ""
echo "1. Initializing database..."
echo "{\"command\":\"init\",\"path\":\"$DB_PATH\"}" | ./target/release/zlf

# Count files
TOTAL=$(find "$WIKI_DIR" -name "*.md" -type f | wc -l)
echo "Found $TOTAL markdown files"

# Import files
COUNT=0
ERRORS=0

echo ""
echo "2. Importing files..."

find "$WIKI_DIR" -name "*.md" -type f | while read -r file; do
    COUNT=$((COUNT + 1))
    
    # Extract filename without path and extension
    filename=$(basename "$file" .md)
    
    # Extract directory structure for labels
    rel_path="${file#$WIKI_DIR/}"
    dir_path=$(dirname "$rel_path")
    
    # Convert directory to labels
    if [ "$dir_path" = "." ]; then
        labels='["article"]'
    else
        # Convert path separators to labels
        labels=$(echo "$dir_path" | tr '/' ',' | sed 's/[^,]*/\"&\"/g' | sed 's/^/[/;s/$/]/')
    fi
    
    # Read file content (first 1000 chars for properties)
    content=$(head -c 1000 "$file" | tr '\n' ' ' | sed 's/"/\\"/g' | sed 's/\\/\\\\/g')
    
    # Create JSON request
    request=$(cat <<EOF
{"command":"add_node","path":"$DB_PATH","labels":$labels,"properties":{"title":"$filename","path":"$rel_path","content":"$content"}}
EOF
)
    
    # Execute import
    result=$(echo "$request" | ./target/release/zlf 2>/dev/null)
    
    if echo "$result" | grep -q '"type":"success"'; then
        if [ $((COUNT % 50)) -eq 0 ]; then
            echo "  Imported $COUNT files..."
        fi
    else
        ERRORS=$((ERRORS + 1))
        if [ $ERRORS -le 5 ]; then
            echo "  Warning: Failed to import $filename"
        fi
    fi
done

echo ""
echo "3. Import complete!"
echo "  Total files: $TOTAL"
echo "  Errors: $ERRORS"

# Verify import
echo ""
echo "4. Verifying import..."
echo "{\"command\":\"query\",\"path\":\"$DB_PATH\",\"query\":\"node(article, X, Props).\"}" | ./target/release/zlf | head -c 500
echo "..."
