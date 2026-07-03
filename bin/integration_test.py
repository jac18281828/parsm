#!/usr/bin/env python3
"""
Parsm Comprehensive Integration Test Suite
Tests all supported formats, operations, and edge cases
"""

import subprocess
import json
import sys
import os
from dataclasses import dataclass
from typing import List
from pathlib import Path


@dataclass
class TestCase:
    name: str
    input_data: str
    args: List[str]
    description: str
    expected_stdout: str
    category: str = "general"
    should_pass: bool = True


class TestRunner:
    def __init__(self):
        self.parsm_binary = self._find_parsm_binary()
        self.passed = 0
        self.failed = 0
        
    def _find_parsm_binary(self) -> str:
        """Find the parsm binary, building if necessary"""
        release_path = Path("target/release/parsm")
        if release_path.exists():
            return str(release_path)
            
        debug_path = Path("target/debug/parsm")
        if debug_path.exists():
            return str(debug_path)
            
        print("Building parsm...")
        subprocess.run(["cargo", "build", "--release"], check=True)
        return str(release_path)
    
    def run_test(self, test_case: TestCase) -> bool:
        """Run a single test case"""
        try:
            cmd = [self.parsm_binary] + test_case.args
            result = subprocess.run(
                cmd,
                input=test_case.input_data,
                text=True,
                capture_output=True,
                timeout=10
            )
            
            exit_ok = (result.returncode == 0) == test_case.should_pass
            output_ok = result.stdout.strip() == test_case.expected_stdout
            success = exit_ok and output_ok

            if success:
                self.passed += 1
                print(f"✅ {test_case.name}: {test_case.description}")
                if result.stdout.strip():
                    print(f"   Output: {result.stdout.strip()}")
            else:
                self.failed += 1
                print(f"❌ {test_case.name}: {test_case.description}")
                if not exit_ok:
                    print(f"   Expected success: {test_case.should_pass}, exit code: {result.returncode}")
                if not output_ok:
                    print(f"   Expected stdout: {test_case.expected_stdout!r}")
                    print(f"   Actual stdout:   {result.stdout.strip()!r}")
                if result.stderr.strip():
                    print(f"   Error: {result.stderr.strip()}")
            
            return success
            
        except Exception as e:
            self.failed += 1
            print(f"❌ {test_case.name}: Exception - {e}")
            return False
    
    def run_category(self, category: str, tests: List[TestCase]):
        """Run all tests in a category"""
        print(f"\n🔧 {category.upper()} TESTS")
        print("=" * 50)
        
        for test in tests:
            self.run_test(test)
    
    def summary(self):
        """Print test summary"""
        total = self.passed + self.failed
        print(f"\n📊 SUMMARY")
        print("=" * 50)
        print(f"Total tests: {total}")
        print(f"Passed: {self.passed}")
        print(f"Failed: {self.failed}")
        
        if self.failed == 0:
            print("🎉 All tests passed!")
            return True
        else:
            print(f"⚠️  {self.failed} tests failed")
            return False


def create_test_cases() -> dict:
    """Create test cases organized by category"""
    
    # JSON Tests
    json_tests = [
        TestCase("json_field_select", '{"name": "Alice", "age": 30}', ["name"], 
                "Extract name field from JSON", expected_stdout='Alice'),
        TestCase("json_nested_field", '{"user": {"name": "Alice"}}', ["user.name"], 
                "Extract nested field from JSON", expected_stdout='Alice'),
        TestCase("json_filter_numeric", '{"name": "Alice", "age": 30}', ["age > 25"], 
                "Filter JSON with numeric comparison", expected_stdout='{"name": "Alice", "age": 30}'),
        TestCase("json_filter_string", '{"name": "Alice", "age": 30}', ['name == "Alice"'], 
                "Filter JSON with string comparison", expected_stdout='{"name": "Alice", "age": 30}'),
        TestCase("json_template_simple", '{"name": "Alice", "age": 30}', ["{${name} is ${age}}"], 
                "Simple template with JSON", expected_stdout='Alice is 30'),
        TestCase("json_template_braced", '{"name": "Alice", "age": 30}', ["{${name} is ${age} years old}"], 
                "Braced template with JSON", expected_stdout='Alice is 30 years old'),
        TestCase("json_filter_template", '{"name": "Alice", "age": 30}', ["age > 25 {Hello ${name}!}"], 
                "Combined filter and template", expected_stdout='Hello Alice!'),
        TestCase("json_array_select", '[{"name": "Alice"}, {"name": "Bob"}]', ["name"], 
                "Field selection from JSON array", expected_stdout='Alice\nBob'),
        TestCase("json_boolean_and", '{"age": 30, "active": true}', ["age > 25 && active == true"], 
                "Boolean AND operation", expected_stdout='{"age": 30, "active": true}'),
        TestCase("json_boolean_or", '{"age": 20, "name": "Alice"}', ['age > 25 || name == "Alice"'], 
                "Boolean OR operation", expected_stdout='{"age": 20, "name": "Alice"}'),
        TestCase("json_string_contains", '{"email": "alice@example.com"}', ['email ~ "@example.com"'], 
                "String contains operation", expected_stdout='{"email": "alice@example.com"}'),
        TestCase("json_null_value", '{"name": null, "age": 30}', ["name"], 
                "Handle null values", expected_stdout='null'),
        TestCase("json_passthrough", '{"name": "Alice"}', [], 
                "JSON passthrough without args", expected_stdout='{"name": "Alice"}'),
    ]
    
    # CSV Tests
    csv_tests = [
        TestCase("csv_field_select", "Alice,30,Engineer", ["field_0"], 
                "Basic CSV field selection", expected_stdout='Alice'),
        TestCase("csv_indexed_select", "Alice,30,Engineer", ["field_2"], 
                "CSV field by index", expected_stdout='Engineer'),
        TestCase("csv_filter_string", "Alice,30,Engineer", ['field_1 > "25"'], 
                "Filter CSV with string comparison", expected_stdout='Alice,30,Engineer'),
        TestCase("csv_template_simple", "Alice,30,Engineer", ["{${field_0} works as ${field_2}}"], 
                "Simple CSV template", expected_stdout='Alice works as Engineer'),
        TestCase("csv_template_indexed", "Alice,30,Engineer", ["{${1} - ${2} - ${3}}"], 
                "Indexed CSV template", expected_stdout='Alice - 30 - Engineer'),
        TestCase("csv_empty_field", "Alice,,Engineer", ["field_1"], 
                "Handle empty CSV field", expected_stdout=''),
        TestCase("csv_multiline", "Alice,30\nBob,25", ["field_0"], 
                "Multi-line CSV processing", expected_stdout='Alice\nBob'),
        
        # Header detection and named field access tests
        TestCase("csv_header_field_select", "name,age,occupation\nTom,45,engineer\nAlice,30,doctor", ["name"], 
                "CSV field selection by header name", expected_stdout='Tom\nAlice'),
        TestCase("csv_header_detection", "name,age,occupation\nTom,45,engineer\nAlice,30,doctor", ["age"], 
                "CSV header detection and skipping", expected_stdout='45\n30'),
        TestCase("csv_no_header_detection", "Tom,45,engineer\nAlice,30,doctor", ["field_0"], 
                "CSV without headers - no skipping", expected_stdout='Tom\nAlice'),
        TestCase("csv_template_headers", "name,age,occupation\nTom,45,engineer\nAlice,30,doctor", ["{${name} is ${age} years old}"], 
                "CSV template with header names", expected_stdout='Tom is 45 years old\nAlice is 30 years old'),
        TestCase("csv_filter_headers", "name,age,occupation\nTom,45,engineer\nAlice,30,doctor\nBob,35,engineer", 
                ["occupation == \"engineer\" {$name}"], 
                "CSV filter with header-based field access", expected_stdout='Tom\nBob'),
        TestCase("csv_mixed_header_patterns", "user_id,firstName,Last_Name,emailAddress\n1,John,Doe,john@example.com\n2,Jane,Smith,jane@example.com", 
                ["firstname"], 
                "CSV with mixed header patterns", expected_stdout='John\nJane'),
    ]
    
    # YAML Tests
    yaml_tests = [
        TestCase("yaml_field_select", "name: Alice\nage: 30", ["name"], 
                "Basic YAML field selection", expected_stdout='Alice'),
        TestCase("yaml_nested_field", "user:\n  name: Alice\n  email: alice@test.com", ["user.name"], 
                "Nested YAML field selection", expected_stdout='Alice'),
        TestCase("yaml_filter", "name: Alice\nage: 30", ["age > 25"], 
                "Filter YAML data", expected_stdout='name: Alice\nage: 30'),
        TestCase("yaml_template", "name: Alice\nage: 30", ["{${name} is ${age}}"], 
                "YAML template rendering", expected_stdout='Alice is 30'),
        TestCase("yaml_document_marker", "---\nname: Alice\nage: 30", ["name"], 
                "YAML with document marker", expected_stdout='Alice'),
        TestCase("yaml_array", "names:\n  - Alice\n  - Bob", ["names"], 
                "YAML array handling", expected_stdout='[\n  "Alice",\n  "Bob"\n]'),
    ]
    
    # TOML Tests
    toml_tests = [
        TestCase("toml_field_select", 'name = "Alice"\nage = 30', ["name"], 
                "Basic TOML field selection", expected_stdout='Alice'),
        TestCase("toml_section", 'name = "Alice"\n\n[profile]\nage = 30', ["profile.age"], 
                "TOML section access", expected_stdout='30'),
        TestCase("toml_filter", 'name = "Alice"\nage = 30', ["age > 25"], 
                "Filter TOML data", expected_stdout='name = "Alice"\nage = 30'),
        TestCase("toml_template", 'name = "Alice"\nage = 30', ["{${name} is ${age}}"], 
                "TOML template rendering", expected_stdout='Alice is 30'),
        TestCase("toml_array", 'name = "Alice"\nhobbies = ["reading", "coding"]', ["hobbies"], 
                "TOML array handling", expected_stdout='[\n  "reading",\n  "coding"\n]'),
    ]
    
    # Logfmt Tests
    logfmt_tests = [
        TestCase("logfmt_field_select", 'level=info msg="User login" user_id=123', ["level"], 
                "Basic logfmt field selection", expected_stdout='info'),
        TestCase("logfmt_quoted_value", 'level=info msg="User login" user="Alice Smith"', ["user"], 
                "Logfmt quoted value extraction", expected_stdout='Alice Smith'),
        TestCase("logfmt_filter", 'level=error msg="Database error" service=api', ['level == "error"'], 
                "Filter logfmt data", expected_stdout='level=error msg="Database error" service=api'),
        TestCase("logfmt_template", 'level=error msg="DB error" service=api', ["{[${level}] ${msg}}"], 
                "Logfmt template rendering", expected_stdout='[error] DB error'),
        TestCase("logfmt_numeric", 'level=info response_time=250 status=200', ["response_time"], 
                "Logfmt numeric field", expected_stdout='250'),
    ]
    
    # Text Tests
    text_tests = [
        TestCase("text_word_select", "Alice 30 Engineer", ["word_0"], 
                "Basic text word selection", expected_stdout='Alice'),
        TestCase("text_word_template", "Alice 30 Engineer", ["{${word_0} is ${word_1}}"], 
                "Text word template", expected_stdout='Alice is 30'),
        TestCase("text_multiword", "Hello world from parsm", ["word_2"], 
                "Multi-word text parsing", expected_stdout='from'),
        TestCase("text_filter", "Alice 30 Engineer", ['word_1 > "25"'], 
                "Filter text data", expected_stdout='Alice 30 Engineer'),
    ]
    
    # Format Detection Tests
    detection_tests = [
        TestCase("detect_json", '{"format": "json"}', ["format"], 
                "Auto-detect JSON format", expected_stdout='json'),
        TestCase("detect_yaml", "format: yaml", ["format"], 
                "Auto-detect YAML format", expected_stdout='yaml'),
        TestCase("detect_toml", 'format = "toml"', ["format"], 
                "Auto-detect TOML format", expected_stdout='toml'),
        TestCase("detect_csv", "col1,col2,col3", ["field_0"], 
                "Auto-detect CSV format", expected_stdout='col1'),
        TestCase("detect_logfmt", "format=logfmt level=info", ["format"], 
                "Auto-detect logfmt format", expected_stdout='logfmt'),
    ]
    
    # Edge Cases
    edge_tests = [
        TestCase("empty_json", "{}", [], 
                "Empty JSON object", expected_stdout='{}'),
        TestCase("malformed_json", '{"name": "Alice"', ["name"], 
                "Malformed JSON (should fall back to text)", should_pass=True, expected_stdout=''),
        TestCase("unicode_text", "café 123 français", ["word_0"], 
                "Unicode text handling", expected_stdout='café'),
        TestCase("special_chars", 'test@domain.com,123,"value with spaces"', ["field_0"], 
                "Special characters in CSV", expected_stdout='test@domain.com'),
    ]
    
    # Streaming Tests
    streaming_tests = [
        TestCase("stream_json", '{"name": "Alice", "age": 30}\n{"name": "Bob", "age": 25}', 
                ["age > 25"], "Stream JSON filtering", expected_stdout='{"name": "Alice", "age": 30}'),
        TestCase("stream_template", '{"name": "Alice"}\n{"name": "Bob"}', 
                ["name"], "Stream template rendering", expected_stdout='Alice\nBob'),
        TestCase("stream_csv", "Alice,30\nBob,25\nCharlie,35", 
                ["field_0"], "Stream CSV processing", expected_stdout='Alice\nBob\nCharlie'),
    ]
    
    # Truthy Operator Tests
    truthy_tests = [
        TestCase("truthy_json_true", '{"active": true, "name": "Alice"}', ["active?"], 
                "Truthy operator with true boolean", expected_stdout='{"active": true, "name": "Alice"}'),
        TestCase("truthy_json_false", '{"active": false, "name": "Alice"}', ["active?"], 
                "Truthy operator with false boolean", expected_stdout=''),
        TestCase("truthy_json_null", '{"active": null, "name": "Alice"}', ["active?"], 
                "Truthy operator with null value", expected_stdout=''),
        TestCase("truthy_json_zero", '{"count": 0, "name": "Alice"}', ["count?"], 
                "Truthy operator with zero", expected_stdout=''),
        TestCase("truthy_json_nonzero", '{"count": 5, "name": "Alice"}', ["count?"], 
                "Truthy operator with non-zero number", expected_stdout='{"count": 5, "name": "Alice"}'),
        TestCase("truthy_json_empty_string", '{"text": "", "name": "Alice"}', ["text?"], 
                "Truthy operator with empty string", expected_stdout=''),
        TestCase("truthy_json_nonempty_string", '{"text": "hello", "name": "Alice"}', ["text?"], 
                "Truthy operator with non-empty string", expected_stdout='{"text": "hello", "name": "Alice"}'),
        TestCase("truthy_csv_present", "Alice,30,Engineer", ["field_1?"], 
                "Truthy operator with CSV field present", expected_stdout='Alice,30,Engineer'),
        TestCase("truthy_csv_empty", "Alice,,Engineer", ["field_1?"], 
                "Truthy operator with CSV empty field", expected_stdout=''),
        TestCase("truthy_template_conditional", '{"active": true, "name": "Alice"}', 
                ["active? {${name} is active}"], 
                "Truthy operator in conditional template", expected_stdout='Alice is active'),
        TestCase("truthy_yaml_present", "active: true\nname: Alice", ["active?"], 
                "Truthy operator with YAML", expected_stdout='active: true\nname: Alice'),
        TestCase("truthy_logfmt_present", "active=true name=Alice", ["active?"], 
                "Truthy operator with logfmt", expected_stdout='active=true name=Alice'),
    ]
    
    # Explicit Format Selection Tests
    format_selection_tests = [
        TestCase("explicit_json", '{"name": "Alice", "age": 30}', ["--json", "name"], 
                "Explicit JSON format selection", expected_stdout='Alice'),
        TestCase("explicit_csv", "Alice,30,Engineer", ["--csv", "field_0"], 
                "Explicit CSV format selection", expected_stdout='Alice'),
        TestCase("explicit_yaml", "name: Alice\nage: 30", ["--yaml", "name"], 
                "Explicit YAML format selection", expected_stdout='Alice'),
        TestCase("explicit_toml", 'name = "Alice"\nage = 30', ["--toml", "name"], 
                "Explicit TOML format selection", expected_stdout='Alice'),
        TestCase("explicit_logfmt", "name=Alice age=30", ["--logfmt", "name"], 
                "Explicit logfmt format selection", expected_stdout='Alice'),
        TestCase("explicit_text", "Alice 30 Engineer", ["--text", "word_0"], 
                "Explicit text format selection", expected_stdout='Alice'),
        TestCase("json_as_csv", '{"name": "Alice", "age": 30}', ["--csv", "field_0"], 
                "Force JSON data to be parsed as CSV", expected_stdout='{"name": "Alice"'),
        TestCase("explicit_json_template", '{"name": "Alice", "age": 30}', 
                ["--json", "{${name} is ${age}}"], 
                "Explicit JSON with template", expected_stdout='Alice is 30'),
        TestCase("explicit_csv_filter", "Alice,30,Engineer\nBob,25,Designer", 
                ["--csv", 'field_1 > "27"'], 
                "Explicit CSV with filter", expected_stdout='Alice,30,Engineer'),
        TestCase("explicit_yaml_nested", "user:\n  name: Alice\n  age: 30", 
                ["--yaml", "user.name"], 
                "Explicit YAML with nested field", expected_stdout='Alice'),
        TestCase("explicit_toml_section", 'name = "Alice"\n\n[profile]\nage = 30', 
                ["--toml", "profile.age"], 
                "Explicit TOML with section", expected_stdout='30'),
    ]
    
    # Regression Tests
    regression_tests = [
        TestCase("field_to_field_comparison",
                '{"score": 98.5, "threshold": 95.0}',
                ["score >= threshold"],
                "Field-to-field numeric comparison (regression for silent-fail bug)",
                expected_stdout='{"score": 98.5, "threshold": 95.0}'),
        TestCase("contains_with_and",
                '{"email": "alice@example.com", "age": 30}',
                ['email ~ "@example.com" && age > 25'],
                "Contains operator combined with && (regression for silent-fail bug)",
                expected_stdout='{"email": "alice@example.com", "age": 30}'),
    ]

    return {
        "json": json_tests,
        "csv": csv_tests,
        "yaml": yaml_tests,
        "toml": toml_tests,
        "logfmt": logfmt_tests,
        "text": text_tests,
        "detection": detection_tests,
        "edge_cases": edge_tests,
        "streaming": streaming_tests,
        "truthy": truthy_tests,
        "format_selection": format_selection_tests,
        "regression": regression_tests,
    }


def main():
    """Main test runner"""
    if not os.path.exists("Cargo.toml"):
        print("Error: This script must be run from the parsm project root directory")
        sys.exit(1)
    
    print("🚀 Parsm Integration Test Suite")
    print("Testing all supported formats and operations")
    
    runner = TestRunner()
    test_categories = create_test_cases()
    
    for category, tests in test_categories.items():
        runner.run_category(category, tests)
    
    success = runner.summary()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
