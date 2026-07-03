#!/bin/bash

# Parsm Quick Test Suite
# Essential functionality validation

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Build the project first
echo -e "${BLUE}Building parsm...${NC}"
cargo build --release || {
    echo -e "${RED}Build failed!${NC}"
    exit 1
}
PARSM="./target/release/parsm"

# Function to run a test
run_test() {
    local test_name="$1"
    local input="$2"
    local args="$3"
    local description="$4"
    local expected="$5"

    TESTS_RUN=$((TESTS_RUN + 1))

    echo -e "${YELLOW}Test $TESTS_RUN: $test_name${NC}"
    echo "  $description"
    echo "  Input: $input"
    echo "  Args: $args"

    local actual
    local actual_stderr
    local exit_code
    local stderr_file
    stderr_file=$(mktemp)
    if [ -n "$args" ]; then
        actual=$(echo -e "$input" | $PARSM "$args" 2>"$stderr_file")
    else
        actual=$(echo -e "$input" | $PARSM 2>"$stderr_file")
    fi
    exit_code=$?
    actual_stderr=$(cat "$stderr_file")
    rm -f "$stderr_file"

    if [ $exit_code -eq 0 ] && [ "$actual" == "$expected" ]; then
        echo -e "  ${GREEN}✓ PASS${NC} - Output: $actual"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "  ${RED}✗ FAIL${NC} - Exit code: $exit_code"
        echo -e "    Expected: $expected"
        echo -e "    Actual:   $actual"
        if [ -n "$actual_stderr" ]; then
            echo -e "    Stderr:   $actual_stderr"
        fi
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    echo
}

echo -e "${BLUE}Starting Parsm Quick Tests${NC}"
echo "============================="
echo

# Essential tests - just verify they work, don't check exact output
echo -e "${BLUE}=== Essential Functionality ===${NC}"

run_test "JSON_FILTER" \
    '{"name": "Alice", "age": 30}' \
    'age > 25' \
    "Basic JSON filtering" \
    '{"name": "Alice", "age": 30}'

run_test "JSON_FIELD_SELECT" \
    '{"name": "Alice", "age": 30}' \
    'name' \
    "JSON field selection" \
    'Alice'

run_test "JSON_TEMPLATE" \
    '{"name": "Alice", "age": 30}' \
    '{${name} is ${age}}' \
    "JSON template" \
    'Alice is 30'

run_test "CSV_FIELD_SELECT" \
    'Alice,30,Engineer' \
    'field_0' \
    "CSV field selection" \
    'Alice'

run_test "CSV_TEMPLATE" \
    'Alice,30,Engineer' \
    '{${1} - ${2} - ${3}}' \
    "CSV indexed template" \
    'Alice - 30 - Engineer'

run_test "YAML_FIELD_SELECT" \
    'name: Alice\nage: 30' \
    'name' \
    "YAML field selection" \
    'Alice'

run_test "TOML_FIELD_SELECT" \
    'name = "Alice"\nage = 30' \
    'name' \
    "TOML field selection" \
    'Alice'

run_test "LOGFMT_FIELD_SELECT" \
    'level=info msg="test" user_id=123' \
    'level' \
    "Logfmt field selection" \
    'info'

run_test "TEXT_FIELD_SELECT" \
    'Alice 30 Engineer' \
    'word_0' \
    "Text word selection" \
    'Alice'

run_test "BOOLEAN_AND" \
    '{"name": "Alice", "age": 30, "active": true}' \
    'age > 25 && active == true' \
    "Boolean AND operation" \
    '{"name": "Alice", "age": 30, "active": true}'

run_test "STRING_CONTAINS" \
    '{"email": "alice@example.com"}' \
    'email ~ "@example.com"' \
    "String contains operation" \
    '{"email": "alice@example.com"}'

run_test "NESTED_FIELD" \
    '{"user": {"name": "Alice"}}' \
    'user.name' \
    "Nested field access" \
    'Alice'

# Multiline format tests
echo -e "${BLUE}=== Multiline Format Tests ===${NC}"

run_test "MULTILINE_JSON" \
    '{"id": 1, "name": "Alice"}\n{"id": 2, "name": "Bob"}\n{"id": 3, "name": "Charlie"}' \
    'id > 1' \
    "Multiline JSON filtering" \
    $'{"id": 2, "name": "Bob"}\n{"id": 3, "name": "Charlie"}'

run_test "MULTILINE_CSV" \
    'name,age,role\nAlice,30,Engineer\nBob,25,Designer\nCharlie,40,Manager' \
    'field_1 > 25' \
    "Multiline CSV filtering" \
    $'Alice,30,Engineer\nCharlie,40,Manager'

run_test "MULTILINE_YAML" \
    '---\nname: Alice\nage: 30\n---\nname: Bob\nage: 25\n---\nname: Charlie\nage: 40' \
    'age > 25' \
    "Multiline YAML filtering" \
    $'age: 30\nage: 40'

run_test "MULTILINE_TOML" \
    '[user1]\nname = "Alice"\nage = 30\n\n[user2]\nname = "Bob"\nage = 25\n\n[user3]\nname = "Charlie"\nage = 40' \
    'age > 25' \
    "Multiline TOML filtering" \
    ''

# Truthy checks and boolean logic tests
echo -e "${BLUE}=== Truthy Checks and Boolean Logic ===${NC}"

run_test "TRUTHY_CHECK" \
    '{"name": "Alice", "active": true}' \
    'active?' \
    "Simple truthy check" \
    '{"name": "Alice", "active": true}'

run_test "TRUTHY_NESTED_CHECK" \
    '{"user": {"verified": true, "name": "Alice"}}' \
    'user.verified?' \
    "Nested truthy check" \
    '{"user": {"verified": true, "name": "Alice"}}'

run_test "TRUTHY_WITH_AND" \
    '{"name": "Alice", "active": true, "premium": true}' \
    'active? && premium?' \
    "Truthy checks with AND" \
    '{"name": "Alice", "active": true, "premium": true}'

run_test "TRUTHY_WITH_OR" \
    '{"name": "Alice", "admin": false, "moderator": true}' \
    'admin? || moderator?' \
    "Truthy checks with OR" \
    '{"name": "Alice", "admin": false, "moderator": true}'

run_test "NEGATED_TRUTHY" \
    '{"name": "Alice", "banned": false}' \
    '!banned?' \
    "Negated truthy check" \
    '{"name": "Alice", "banned": false}'

run_test "COMPLEX_BOOLEAN" \
    '{"name": "Alice", "age": 30, "verified": true, "role": "user"}' \
    '(age > 25 && verified?) || role == "admin"' \
    "Complex boolean logic" \
    '{"name": "Alice", "age": 30, "verified": true, "role": "user"}'

run_test "NO_ARGS_PASSTHROUGH" \
    '{"name": "Alice"}' \
    '' \
    "No arguments passthrough" \
    '{"name": "Alice"}'

# Comprehensive operator tests
echo -e "${BLUE}=== Comprehensive Operator Tests ===${NC}"

run_test "EQUAL_OPERATOR" \
    '{"name": "Alice", "age": 30}' \
    'name == "Alice"' \
    "Equality operator (==)" \
    '{"name": "Alice", "age": 30}'

run_test "NOT_EQUAL_OPERATOR" \
    '{"name": "Alice", "age": 30}' \
    'name != "Bob"' \
    "Not equal operator (!=)" \
    '{"name": "Alice", "age": 30}'

run_test "LESS_THAN_OPERATOR" \
    '{"name": "Alice", "age": 30}' \
    'age < 35' \
    "Less than operator (<)" \
    '{"name": "Alice", "age": 30}'

run_test "LESS_EQUAL_OPERATOR" \
    '{"name": "Alice", "age": 30}' \
    'age <= 30' \
    "Less than or equal operator (<=)" \
    '{"name": "Alice", "age": 30}'

run_test "GREATER_THAN_OPERATOR" \
    '{"name": "Alice", "age": 30}' \
    'age > 25' \
    "Greater than operator (>)" \
    '{"name": "Alice", "age": 30}'

run_test "GREATER_EQUAL_OPERATOR" \
    '{"name": "Alice", "age": 30}' \
    'age >= 30' \
    "Greater than or equal operator (>=)" \
    '{"name": "Alice", "age": 30}'

run_test "CONTAINS_OPERATOR" \
    '{"email": "alice@example.com"}' \
    'email *= "@example"' \
    "Contains operator (*=)" \
    '{"email": "alice@example.com"}'

run_test "STARTS_WITH_OPERATOR" \
    '{"email": "alice@example.com"}' \
    'email ^= "alice"' \
    "Starts with operator (^=)" \
    '{"email": "alice@example.com"}'

run_test "ENDS_WITH_OPERATOR" \
    '{"email": "alice@example.com"}' \
    'email $= ".com"' \
    "Ends with operator ($=)" \
    '{"email": "alice@example.com"}'

run_test "REGEX_OPERATOR" \
    '{"email": "alice@example.com", "phone": "123-456-7890"}' \
    'email ~= "@.*\.com$"' \
    "Regex operator (~=) - email pattern" \
    '{"email": "alice@example.com", "phone": "123-456-7890"}'

run_test "REGEX_OPERATOR_PHONE" \
    '{"email": "alice@example.com", "phone": "123-456-7890"}' \
    'phone ~= "\\d{3}-\\d{3}-\\d{4}"' \
    "Regex operator (~=) - phone pattern" \
    '{"email": "alice@example.com", "phone": "123-456-7890"}'

run_test "REGEX_OPERATOR_CASE_INSENSITIVE" \
    '{"name": "ALICE"}' \
    'name ~= "(?i)alice"' \
    "Regex operator (~=) - case insensitive" \
    '{"name": "ALICE"}'

# Operator tests with different data types
echo -e "${BLUE}=== Operator Tests with Different Data Types ===${NC}"

run_test "EQUAL_BOOLEAN" \
    '{"active": true, "verified": false}' \
    'active == true' \
    "Boolean equality" \
    '{"active": true, "verified": false}'

run_test "NOT_EQUAL_BOOLEAN" \
    '{"active": true, "verified": false}' \
    'verified != true' \
    "Boolean not equal" \
    '{"active": true, "verified": false}'

run_test "EQUAL_NUMBER_DECIMAL" \
    '{"score": 98.5, "threshold": 95.0}' \
    'score >= threshold' \
    "Decimal number comparison" \
    '{"score": 98.5, "threshold": 95.0}'

run_test "STRING_NUMBER_COMPARISON" \
    '{"age": "30", "limit": 25}' \
    'age > limit' \
    "String to number comparison" \
    '{"age": "30", "limit": 25}'

run_test "CONTAINS_NUMBER_AS_STRING" \
    '{"id": 12345}' \
    'id *= "234"' \
    "Contains operator with number field" \
    '{"id": 12345}'

# Operator precedence and spacing tests
echo -e "${BLUE}=== Operator Precedence and Spacing Tests ===${NC}"

run_test "NO_SPACES_EQUAL" \
    '{"age": 30}' \
    'age==30' \
    "Equality without spaces" \
    '{"age": 30}'

run_test "NO_SPACES_NOT_EQUAL" \
    '{"age": 30}' \
    'age!=25' \
    "Not equal without spaces" \
    '{"age": 30}'

run_test "NO_SPACES_LESS_EQUAL" \
    '{"age": 30}' \
    'age<=30' \
    "Less than or equal without spaces" \
    '{"age": 30}'

run_test "NO_SPACES_GREATER_EQUAL" \
    '{"age": 30}' \
    'age>=30' \
    "Greater than or equal without spaces" \
    '{"age": 30}'

run_test "SPACES_REQUIRED_LESS_THAN" \
    '{"age": 30}' \
    'age < 35' \
    "Less than with required spaces" \
    '{"age": 30}'

run_test "SPACES_REQUIRED_GREATER_THAN" \
    '{"age": 30}' \
    'age > 25' \
    "Greater than with required spaces" \
    '{"age": 30}'

# Complex operator combinations
echo -e "${BLUE}=== Complex Operator Combinations ===${NC}"

run_test "AND_WITH_DIFFERENT_OPERATORS" \
    '{"name": "Alice", "age": 30, "email": "alice@example.com"}' \
    'age >= 18 && email *= "@example"' \
    "AND with different operators" \
    '{"name": "Alice", "age": 30, "email": "alice@example.com"}'

run_test "OR_WITH_STRING_OPERATORS" \
    '{"name": "Alice", "role": "admin"}' \
    'name ^= "Al" || role $= "min"' \
    "OR with string operators" \
    '{"name": "Alice", "role": "admin"}'

run_test "MIXED_OPERATOR_PRECEDENCE" \
    '{"score": 85, "bonus": 10, "name": "Alice"}' \
    'score > 80 && bonus >= 5 && name != "Bob"' \
    "Mixed operator types with precedence" \
    '{"score": 85, "bonus": 10, "name": "Alice"}'

run_test "REGEX_WITH_BOOLEAN_LOGIC" \
    '{"email": "alice@company.com", "verified": true}' \
    'email ~= "@company\\." && verified == true' \
    "Regex with boolean logic" \
    '{"email": "alice@company.com", "verified": true}'

# Edge cases and special values
echo -e "${BLUE}=== Operator Edge Cases ===${NC}"

run_test "NULL_COMPARISON" \
    '{"value": null, "name": "Alice"}' \
    'value == null' \
    "Null value comparison" \
    '{"value": null, "name": "Alice"}'

run_test "EMPTY_STRING_CONTAINS" \
    '{"text": "hello world"}' \
    'text *= ""' \
    "Contains with empty string" \
    '{"text": "hello world"}'

run_test "REGEX_FALLBACK_INVALID" \
    '{"text": "hello"}' \
    'text ~= "[invalid"' \
    "Regex with invalid pattern (should fallback)" \
    ''

run_test "CROSS_TYPE_STRING_NUMBER" \
    '{"version": "1.5", "min_version": 1.2}' \
    'version > min_version' \
    "Cross-type string/number comparison" \
    '{"version": "1.5", "min_version": 1.2}'

# Nested field operator tests
echo -e "${BLUE}=== Nested Field Operator Tests ===${NC}"

run_test "NESTED_FIELD_EQUAL" \
    '{"user": {"profile": {"age": 30}}}' \
    'user.profile.age == 30' \
    "Nested field equality" \
    '{"user": {"profile": {"age": 30}}}'

run_test "NESTED_FIELD_REGEX" \
    '{"user": {"contact": {"email": "alice@example.com"}}}' \
    'user.contact.email ~= "@example"' \
    "Nested field regex" \
    '{"user": {"contact": {"email": "alice@example.com"}}}'

run_test "ARRAY_INDEX_OPERATOR" \
    '{"scores": [85, 92, 78]}' \
    'scores.1 > 90' \
    "Array index with operator" \
    '{"scores": [85, 92, 78]}'

# Additional edge case and error handling tests
echo -e "${BLUE}=== Additional Edge Cases and Error Handling ===${NC}"

run_test "OPERATOR_WITH_MISSING_FIELD" \
    '{"name": "Alice"}' \
    'missing_field == "value"' \
    "Operator with missing field (should filter out)" \
    ''

run_test "REGEX_COMPLEX_PATTERN" \
    '{"url": "https://example.com/api/v1/users"}' \
    'url ~= "https://[^/]+/api/v\\d+/"' \
    "Complex regex pattern" \
    '{"url": "https://example.com/api/v1/users"}'

run_test "MULTIPLE_REGEX_FLAGS" \
    '{"text": "Hello\nWorld"}' \
    'text ~= "(?ims)hello.*world"' \
    "Regex with multiple flags" \
    ''

run_test "OPERATOR_CHAINING" \
    '{"name": "Alice", "email": "alice@example.com", "age": 30}' \
    'name == "Alice" && email *= "@example" && age >= 25' \
    "Multiple operator chaining" \
    '{"name": "Alice", "email": "alice@example.com", "age": 30}'

run_test "CONTAINS_SPECIAL_CHARS" \
    '{"path": "/api/v1/users?id=123&active=true"}' \
    'path *= "?id="' \
    "Contains with special characters" \
    '{"path": "/api/v1/users?id=123&active=true"}'

run_test "STARTS_WITH_EMPTY" \
    '{"text": "hello"}' \
    'text ^= ""' \
    "Starts with empty string" \
    '{"text": "hello"}'

run_test "ENDS_WITH_FULL_STRING" \
    '{"word": "hello"}' \
    'word $= "hello"' \
    "Ends with full string match" \
    '{"word": "hello"}'

run_test "NUMERIC_STRING_EQUAL" \
    '{"version": "2.0", "target": "2.0"}' \
    'version == target' \
    "Numeric string equality" \
    '{"version": "2.0", "target": "2.0"}'

run_test "BOOLEAN_STRING_MIXED" \
    '{"flag": "true", "active": true}' \
    'flag == "true" && active == true' \
    "Boolean and string boolean mixed" \
    '{"flag": "true", "active": true}'

run_test "ZERO_COMPARISON" \
    '{"count": 0, "limit": 10}' \
    'count < limit && count >= 0' \
    "Zero value comparisons" \
    '{"count": 0, "limit": 10}'

run_test "STRING_CONTAINS_WITH_AND" \
    '{"email": "alice@example.com", "age": 30}' \
    'email ~ "@example.com" && age > 25' \
    "Contains operator combined with && (regression for silent-fail bug)" \
    '{"email": "alice@example.com", "age": 30}'

# Summary
echo
echo -e "${BLUE}==============================${NC}"
echo -e "${BLUE}Quick Test Summary${NC}"
echo -e "${BLUE}==============================${NC}"
echo "Total tests run: $TESTS_RUN"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All essential functionality working! ✅${NC}"
    exit 0
else
    echo -e "${YELLOW}Some functionality may need attention.${NC}"
    exit 1
fi
