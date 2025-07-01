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
            
            success = (result.returncode == 0) == test_case.should_pass
            
            if success:
                self.passed += 1
                print(f"✅ {test_case.name}: {test_case.description}")
                if result.stdout.strip():
                    print(f"   Output: {result.stdout.strip()}")
            else:
                self.failed += 1
                print(f"❌ {test_case.name}: {test_case.description}")
                print(f"   Expected success: {test_case.should_pass}")
                print(f"   Exit code: {result.returncode}")
                if result.stdout.strip():
                    print(f"   Output: {result.stdout.strip()}")
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
                "Extract name field from JSON"),
        TestCase("json_nested_field", '{"user": {"name": "Alice"}}', ["user.name"], 
                "Extract nested field from JSON"),
        TestCase("json_filter_numeric", '{"name": "Alice", "age": 30}', ["age > 25"], 
                "Filter JSON with numeric comparison"),
        TestCase("json_filter_string", '{"name": "Alice", "age": 30}', ['name == "Alice"'], 
                "Filter JSON with string comparison"),
        TestCase("json_template_simple", '{"name": "Alice", "age": 30}', ["$name is $age"], 
                "Simple template with JSON"),
        TestCase("json_template_braced", '{"name": "Alice", "age": 30}', ["{${name} is ${age} years old}"], 
                "Braced template with JSON"),
        TestCase("json_filter_template", '{"name": "Alice", "age": 30}', ["age > 25 {Hello ${name}!}"], 
                "Combined filter and template"),
        TestCase("json_array_select", '[{"name": "Alice"}, {"name": "Bob"}]', ["name"], 
                "Field selection from JSON array"),
        TestCase("json_boolean_and", '{"age": 30, "active": true}', ["age > 25 && active == true"], 
                "Boolean AND operation"),
        TestCase("json_boolean_or", '{"age": 20, "name": "Alice"}', ['age > 25 || name == "Alice"'], 
                "Boolean OR operation"),
        TestCase("json_string_contains", '{"email": "alice@example.com"}', ['email ~ "@example.com"'], 
                "String contains operation"),
        TestCase("json_null_value", '{"name": null, "age": 30}', ["name"], 
                "Handle null values"),
        TestCase("json_passthrough", '{"name": "Alice"}', [], 
                "JSON passthrough without args"),
    ]
    
    # CSV Tests
    csv_tests = [
        TestCase("csv_field_select", "Alice,30,Engineer", ["field_0"], 
                "Basic CSV field selection"),
        TestCase("csv_indexed_select", "Alice,30,Engineer", ["field_2"], 
                "CSV field by index"),
        TestCase("csv_filter_string", "Alice,30,Engineer", ['field_1 > "25"'], 
                "Filter CSV with string comparison"),
        TestCase("csv_template_simple", "Alice,30,Engineer", ["$field_0 works as $field_2"], 
                "Simple CSV template"),
        TestCase("csv_template_indexed", "Alice,30,Engineer", ["{${1} - ${2} - ${3}}"], 
                "Indexed CSV template"),
        TestCase("csv_empty_field", "Alice,,Engineer", ["field_1"], 
                "Handle empty CSV field"),
        TestCase("csv_multiline", "Alice,30\nBob,25", ["field_0"], 
                "Multi-line CSV processing"),
    ]
    
    # YAML Tests
    yaml_tests = [
        TestCase("yaml_field_select", "name: Alice\nage: 30", ["name"], 
                "Basic YAML field selection"),
        TestCase("yaml_nested_field", "user:\n  name: Alice\n  email: alice@test.com", ["user.name"], 
                "Nested YAML field selection"),
        TestCase("yaml_filter", "name: Alice\nage: 30", ["age > 25"], 
                "Filter YAML data"),
        TestCase("yaml_template", "name: Alice\nage: 30", ["{${name} is ${age}}"], 
                "YAML template rendering"),
        TestCase("yaml_document_marker", "---\nname: Alice\nage: 30", ["name"], 
                "YAML with document marker"),
        TestCase("yaml_array", "names:\n  - Alice\n  - Bob", ["names"], 
                "YAML array handling"),
    ]
    
    # TOML Tests
    toml_tests = [
        TestCase("toml_field_select", 'name = "Alice"\nage = 30', ["name"], 
                "Basic TOML field selection"),
        TestCase("toml_section", 'name = "Alice"\n\n[profile]\nage = 30', ["profile.age"], 
                "TOML section access"),
        TestCase("toml_filter", 'name = "Alice"\nage = 30', ["age > 25"], 
                "Filter TOML data"),
        TestCase("toml_template", 'name = "Alice"\nage = 30', ["{${name} is ${age}}"], 
                "TOML template rendering"),
        TestCase("toml_array", 'name = "Alice"\nhobbies = ["reading", "coding"]', ["hobbies"], 
                "TOML array handling"),
    ]
    
    # Logfmt Tests
    logfmt_tests = [
        TestCase("logfmt_field_select", 'level=info msg="User login" user_id=123', ["level"], 
                "Basic logfmt field selection"),
        TestCase("logfmt_quoted_value", 'level=info msg="User login" user="Alice Smith"', ["user"], 
                "Logfmt quoted value extraction"),
        TestCase("logfmt_filter", 'level=error msg="Database error" service=api', ['level == "error"'], 
                "Filter logfmt data"),
        TestCase("logfmt_template", 'level=error msg="DB error" service=api', ["{[${level}] ${msg}}"], 
                "Logfmt template rendering"),
        TestCase("logfmt_numeric", 'level=info response_time=250 status=200', ["response_time"], 
                "Logfmt numeric field"),
    ]
    
    # Text Tests
    text_tests = [
        TestCase("text_word_select", "Alice 30 Engineer", ["word_0"], 
                "Basic text word selection"),
        TestCase("text_word_template", "Alice 30 Engineer", ["$word_0 is $word_1"], 
                "Text word template"),
        TestCase("text_multiword", "Hello world from parsm", ["word_2"], 
                "Multi-word text parsing"),
        TestCase("text_filter", "Alice 30 Engineer", ['word_1 > "25"'], 
                "Filter text data"),
    ]
    
    # Format Detection Tests
    detection_tests = [
        TestCase("detect_json", '{"format": "json"}', ["format"], 
                "Auto-detect JSON format"),
        TestCase("detect_yaml", "format: yaml", ["format"], 
                "Auto-detect YAML format"),
        TestCase("detect_toml", 'format = "toml"', ["format"], 
                "Auto-detect TOML format"),
        TestCase("detect_csv", "col1,col2,col3", ["field_0"], 
                "Auto-detect CSV format"),
        TestCase("detect_logfmt", "format=logfmt level=info", ["format"], 
                "Auto-detect logfmt format"),
    ]
    
    # Edge Cases
    edge_tests = [
        TestCase("empty_json", "{}", [], 
                "Empty JSON object"),
        TestCase("malformed_json", '{"name": "Alice"', ["name"], 
                "Malformed JSON (should fall back to text)", should_pass=True),
        TestCase("unicode_text", "café 123 français", ["word_0"], 
                "Unicode text handling"),
        TestCase("special_chars", 'test@domain.com,123,"value with spaces"', ["field_0"], 
                "Special characters in CSV"),
    ]
    
    # Streaming Tests
    streaming_tests = [
        TestCase("stream_json", '{"name": "Alice", "age": 30}\n{"name": "Bob", "age": 25}', 
                ["age > 25"], "Stream JSON filtering"),
        TestCase("stream_template", '{"name": "Alice"}\n{"name": "Bob"}', 
                ["$name"], "Stream template rendering"),
        TestCase("stream_csv", "Alice,30\nBob,25\nCharlie,35", 
                ["field_0"], "Stream CSV processing"),
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
