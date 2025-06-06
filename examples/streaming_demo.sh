#!/bin/bash

# Streaming Parser Demo
# This script demonstrates how parsm can handle different data formats in streaming mode

echo "=== PARSM Streaming Parser Demo ==="
echo

# Build the project first
cargo build --release

PARSM="./target/release/parsm"

echo "1. JSON streaming (NDJSON format):"
echo '{"name": "Alice", "age": 30, "city": "NYC"}'
echo '{"name": "Bob", "age": 25, "city": "LA"}'
echo '{"name": "Charlie", "age": 35, "city": "Chicago"}'
echo | {
    echo '{"name": "Alice", "age": 30, "city": "NYC"}'
    echo '{"name": "Bob", "age": 25, "city": "LA"}'
    echo '{"name": "Charlie", "age": 35, "city": "Chicago"}'
} | $PARSM
echo

echo "2. CSV streaming:"
echo "Name,Age,City"
echo "Alice,30,NYC"
echo "Bob,25,LA"
echo "Charlie,35,Chicago"
echo | {
    echo "Name,Age,City"
    echo "Alice,30,NYC"
    echo "Bob,25,LA"
    echo "Charlie,35,Chicago"
} | $PARSM
echo

echo "3. Logfmt streaming (structured logs):"
echo 'timestamp=2025-06-05T10:00:00 level=info msg="Server started" port=8080'
echo 'timestamp=2025-06-05T10:01:00 level=warn msg="High memory usage" usage=85%'
echo 'timestamp=2025-06-05T10:02:00 level=error msg="Database timeout" query=SELECT'
echo | {
    echo 'timestamp=2025-06-05T10:00:00 level=info msg="Server started" port=8080'
    echo 'timestamp=2025-06-05T10:01:00 level=warn msg="High memory usage" usage=85%'
    echo 'timestamp=2025-06-05T10:02:00 level=error msg="Database timeout" query=SELECT'
} | $PARSM
echo

echo "4. YAML streaming:"
echo "name: Alice"
echo "name: Bob"
echo "name: Charlie"
echo | {
    echo "name: Alice"
    echo "name: Bob"
    echo "name: Charlie"
} | $PARSM
echo

echo "5. TOML streaming:"
echo 'name = "Alice"'
echo 'name = "Bob"'
echo 'name = "Charlie"'
echo | {
    echo 'name = "Alice"'
    echo 'name = "Bob"'
    echo 'name = "Charlie"'
} | $PARSM
echo

echo "6. Plain text streaming (space-separated words):"
echo "the quick brown fox"
echo "jumps over the lazy dog"
echo "hello world from parsm"
echo | {
    echo "the quick brown fox"
    echo "jumps over the lazy dog"
    echo "hello world from parsm"
} | $PARSM
echo

echo "7. Streaming with pipes (simulating large dataset):"
echo "Generating 1000 JSON records and processing them through parsm..."
for i in {1..1000}; do
    echo "{\"id\": $i, \"user\": \"user$i\", \"active\": true}"
done | $PARSM | head -5
echo "... (showing first 5 of 1000 records)"
echo

echo "8. Format detection demonstration:"
echo "parsm automatically detects the format from the first line:"
echo
echo "Input: {\"format\": \"json\"}"
echo "{\"format\": \"json\"}" | $PARSM
echo
echo "Input: format,type"
echo "format,type" | $PARSM
echo
echo "Input: level=info msg=test"
echo "level=info msg=test" | $PARSM
echo
echo "Input: hello world text"
echo "hello world text" | $PARSM
echo

echo "Demo complete! parsm successfully handles streaming data in multiple formats."
