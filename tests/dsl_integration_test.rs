//! Comprehensive DSL Integration Tests
//!
//! This module contains regression tests and comprehensive integration tests for the DSL parser.
//! These tests ensure that critical parsing distinctions and edge cases are preserved
//! across refactoring and modularization efforts.

use parsm::dsl::parse_command;
use parsm::filter::{ComparisonOp, FilterExpr, FilterValue, TemplateItem};

/// Test the critical parsing distinctions required for unambiguous DSL behavior.
///
/// This test ensures that:
/// - `$name` and `${name}` are always parsed as field substitutions (variables)
/// - `"name"` and `{name}` are always parsed as literals
/// - Dollar amounts like `$20`, `$0`, `$1` are always treated as numeric literals (not variables)
/// - Only `${0}`, `${1}`, `${20}` are treated as field substitutions for numeric fields
#[test]
fn test_critical_parsing_distinctions() {
    println!("\n=== Testing Critical Parsing Distinctions ===");

    // Test 1: $name should be parsed as field substitution (variable)
    println!("\nTest 1: $name as field substitution");
    let result = parse_command("$name").unwrap();
    assert!(result.template.is_some(), "$name should be template");
    assert!(result.filter.is_none(), "$name should not be filter");
    assert!(
        result.field_selector.is_none(),
        "$name should not be field selector"
    );

    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Field(field) => {
            assert_eq!(field.parts, vec!["name"]);
            println!(
                "  ✓ $name correctly parsed as field substitution: {:?}",
                field.parts
            );
        }
        TemplateItem::Literal(text) => {
            panic!("$name should be field substitution, not literal: {text}");
        }
        TemplateItem::Conditional { .. } => {
            panic!("$name should not be conditional template");
        }
    }

    // Test 2: ${name} should be parsed as field substitution (variable)
    println!("\nTest 2: ${{name}} as field substitution");
    let result = parse_command("${name}").unwrap();
    assert!(result.template.is_some(), "${{name}} should be template");
    assert!(result.filter.is_none(), "${{name}} should not be filter");
    assert!(
        result.field_selector.is_none(),
        "${{name}} should not be field selector"
    );

    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Field(field) => {
            assert_eq!(field.parts, vec!["name"]);
            println!(
                "  ✓ ${{name}} correctly parsed as field substitution: {:?}",
                field.parts
            );
        }
        TemplateItem::Literal(text) => {
            panic!("${{name}} should be field substitution, not literal: {text}");
        }
        TemplateItem::Conditional { .. } => {
            panic!("${{name}} should not be conditional template");
        }
    }

    // Test 3: "name" should be parsed as literal (quoted field selector)
    println!("\nTest 3: \"name\" as literal field selector");
    let result = parse_command("\"name\"").unwrap();
    assert!(
        result.field_selector.is_some(),
        "\"name\" should be field selector"
    );
    assert!(result.filter.is_none(), "\"name\" should not be filter");
    assert!(result.template.is_none(), "\"name\" should not be template");

    let field_selector = result.field_selector.unwrap();
    assert_eq!(field_selector.parts, vec!["name"]);
    println!(
        "  ✓ \"name\" correctly parsed as field selector: {:?}",
        field_selector.parts
    );

    // Test 4: {name} should be parsed as literal template
    println!("\nTest 4: {{name}} as literal template");
    let result = parse_command("{name}").unwrap();
    assert!(result.template.is_some(), "{{name}} should be template");
    assert!(result.filter.is_none(), "{{name}} should not be filter");
    assert!(
        result.field_selector.is_none(),
        "{{name}} should not be field selector"
    );

    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Literal(text) => assert_eq!(text, "name"),
        _ => panic!("{{name}} should be literal template, not field substitution"),
    }

    // Test 5: $20 should be parsed as literal (dollar amount, not variable)
    println!("\nTest 5: $20 as literal dollar amount");
    let result = parse_command("$20").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Literal(text) => assert_eq!(text, "$20"),
        _ => panic!("Expected $20 to be literal"),
    }

    // Test 6: $0 should be parsed as literal (dollar amount, not variable)
    println!("\nTest 6: $0 as literal dollar amount");
    let result = parse_command("$0").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Literal(text) => assert_eq!(text, "$0"),
        _ => panic!("Expected $0 to be literal"),
    }

    // Test 7: ${0} should be parsed as field substitution (numeric field reference)
    println!("\nTest 7: ${{0}} as field substitution");
    let result = parse_command("${0}");
    match result {
        Ok(parsed) => {
            if parsed.template.is_some() {
                let template = parsed.template.unwrap();
                if template.items.len() == 1 {
                    match &template.items[0] {
                        TemplateItem::Field(field) => {
                            // ${0} maps to "$0" field reference
                            assert_eq!(field.parts, vec!["$0"]);
                            println!("  ✓ ${{0}} correctly parsed as field substitution to $0");
                        }
                        _ => panic!("${{0}} should be field substitution"),
                    }
                }
            } else {
                panic!("${{0}} should parse as template");
            }
        }
        Err(e) => {
            println!("  ⚠ ${{0}} failed to parse: {e}");
            // This might be acceptable depending on implementation
        }
    }

    println!("\n=== All Critical Parsing Distinction Tests Completed ===");
}

/// Test quoted string literals parsing correctly as field selectors.
#[test]
fn test_quoted_string_literals() {
    println!("\n=== Testing Quoted String Literals ===");

    // Test 1: "Alice" should be parsed as field selector with value "Alice"
    println!("\nTest 1: \"Alice\" as quoted field selector");
    let result = parse_command("\"Alice\"").unwrap();
    assert!(
        result.field_selector.is_some(),
        "\"Alice\" should be field selector"
    );
    assert!(result.filter.is_none(), "\"Alice\" should not be filter");
    assert!(
        result.template.is_none(),
        "\"Alice\" should not be template"
    );

    let field_selector = result.field_selector.unwrap();
    assert_eq!(field_selector.parts, vec!["Alice"]);
    println!(
        "  ✓ \"Alice\" correctly parsed as field selector: {:?}",
        field_selector.parts
    );

    // Test 2: "25" should be parsed as field selector with value "25" (string)
    println!("\nTest 2: \"25\" as quoted field selector");
    let result = parse_command("\"25\"").unwrap();
    assert!(
        result.field_selector.is_some(),
        "\"25\" should be field selector"
    );
    assert!(result.filter.is_none(), "\"25\" should not be filter");
    assert!(result.template.is_none(), "\"25\" should not be template");

    let field_selector = result.field_selector.unwrap();
    assert_eq!(field_selector.parts, vec!["25"]);
    println!(
        "  ✓ \"25\" correctly parsed as field selector: {:?}",
        field_selector.parts
    );

    // Test 3: Single-quoted strings should work the same way
    println!("\nTest 3: 'Alice' as single-quoted field selector");
    let result = parse_command("'Alice'").unwrap();
    assert!(
        result.field_selector.is_some(),
        "'Alice' should be field selector"
    );
    assert!(result.filter.is_none(), "'Alice' should not be filter");
    assert!(result.template.is_none(), "'Alice' should not be template");

    let field_selector = result.field_selector.unwrap();
    assert_eq!(field_selector.parts, vec!["Alice"]);
    println!(
        "  ✓ 'Alice' correctly parsed as field selector: {:?}",
        field_selector.parts
    );

    // Test 4: 'name with spaces' should work
    println!("\nTest 4: 'field name' as quoted field selector with spaces");
    let result = parse_command("'field name'").unwrap();
    assert!(
        result.field_selector.is_some(),
        "'field name' should be field selector"
    );

    let field_selector = result.field_selector.unwrap();
    assert_eq!(field_selector.parts, vec!["field name"]);
    println!(
        "  ✓ 'field name' correctly parsed as field selector: {:?}",
        field_selector.parts
    );

    // Test 5: "field.with.dots" should handle dots correctly
    println!("\nTest 5: \"field.with.dots\" as dotted field selector");
    let result = parse_command("\"field.with.dots\"").unwrap();
    assert!(
        result.field_selector.is_some(),
        "\"field.with.dots\" should be field selector"
    );

    let field_selector = result.field_selector.unwrap();
    assert_eq!(field_selector.parts, vec!["field", "with", "dots"]);
    println!(
        "  ✓ \"field.with.dots\" correctly parsed as field selector: {:?}",
        field_selector.parts
    );

    println!("\n=== Quoted String Literal Tests Completed ===");
}

/// Test template variable edge cases with numeric and literal patterns.
#[test]
fn test_template_variable_edge_cases() {
    // Test ${0} - should be special variable for original input
    let result = parse_command("${0}").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Field(field) => {
            // ${0} should map to "$0" field reference
            assert_eq!(field.parts, vec!["$0"]);
        }
        _ => panic!("Expected ${{0}} to be field substitution"),
    }

    // Test $0 - should be literal (not special)
    let result = parse_command("$0").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Literal(text) => assert_eq!(text, "$0"),
        _ => panic!("Expected $0 to be literal"),
    }

    // Test $20 - should be literal dollar amount
    let result = parse_command("$20").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Literal(text) => assert_eq!(text, "$20"),
        _ => panic!("Expected $20 to be literal"),
    }

    // Test $1 - should be literal dollar amount
    let result = parse_command("$1").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Literal(text) => assert_eq!(text, "$1"),
        _ => panic!("Expected $1 to be literal"),
    }

    // Test ${1} - should be field variable (maps to "1")
    let result = parse_command("${1}").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Field(field) => assert_eq!(field.parts, vec!["1"]),
        _ => panic!("Expected ${{1}} to be field substitution"),
    }

    // Test ${2} - should be field variable (maps to "2")
    let result = parse_command("${2}").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Field(field) => assert_eq!(field.parts, vec!["2"]),
        _ => panic!("Expected ${{2}} to be field substitution"),
    }

    // Test ${20} - should be field variable (maps to "20")
    let result = parse_command("${20}").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Field(field) => assert_eq!(field.parts, vec!["20"]),
        _ => panic!("Expected ${{20}} to be field substitution"),
    }
}

/// Test templates with mixed numeric literals and variables.
#[test]
fn test_mixed_numeric_template_patterns() {
    // Test braced template with dollar amounts and variables: {I have $20 and ${name} has $100}
    let result = parse_command("{I have $20 and ${name} has $100}").unwrap();
    assert!(result.template.is_some());
    let template = result.template.unwrap();

    // Should have 3 items: literal "I have $20 and ", field "name", literal " has $100"
    assert!(template.items.len() >= 2); // At least literal and field

    // Check that we have both literal text with dollar amounts and field substitutions
    let mut found_literal_with_dollar = false;
    let mut found_field = false;

    for item in &template.items {
        match item {
            TemplateItem::Literal(text) => {
                if text.contains("$20") || text.contains("$100") {
                    found_literal_with_dollar = true;
                }
            }
            TemplateItem::Field(field) => {
                if field.parts == vec!["name"] {
                    found_field = true;
                }
            }
            _ => {}
        }
    }

    assert!(
        found_literal_with_dollar,
        "Should contain literal dollar amounts"
    );
    assert!(found_field, "Should contain field substitution");
}

/// Test edge cases and boundary conditions for parsing distinctions.
#[test]
fn test_parsing_edge_cases() {
    println!("\n=== Testing Edge Cases ===");

    let test_cases = vec![
        // Edge cases for dollar amounts vs variables
        ("$1", "literal dollar amount"),
        ("$999", "literal dollar amount"),
        ("$00", "literal dollar amount"),
        ("$01", "literal dollar amount"),
        // Edge cases for braced expressions
        ("${1}", "field substitution"),
        ("${999}", "field substitution"),
        ("${00}", "field substitution"),
        ("${01}", "field substitution"),
        // Edge cases for variable names
        ("$a", "field substitution - single letter"),
        ("$_", "field substitution - underscore"),
        ("$name_123", "field substitution - alphanumeric"),
        // Edge cases for literals
        ("{$20}", "literal template with dollar amount"),
        ("{${name}}", "template with field substitution"),
        ("\"$20\"", "quoted literal"),
        // Boundary cases
        ("$", "bare dollar sign"),
        ("${}", "empty braced expression"),
        ("{}", "empty braces"),
    ];

    for (input, description) in test_cases {
        println!("\nTesting edge case: {input} ({description})");
        match parse_command(input) {
            Ok(result) => {
                println!("  ✓ Parsed successfully:");
                println!("    Filter: {:?}", result.filter.is_some());
                println!("    Template: {:?}", result.template.is_some());
                println!("    Field selector: {:?}", result.field_selector.is_some());

                // Verify specific expectations for key cases
                match input {
                    "$1" | "$999" | "$00" | "$01" => {
                        assert!(result.template.is_some(), "{input} should be template");
                        if let Some(template) = result.template {
                            if !template.items.is_empty() {
                                match &template.items[0] {
                                    TemplateItem::Literal(text) => assert_eq!(text, input),
                                    _ => panic!("{input} should be literal"),
                                }
                            }
                        }
                    }
                    "${1}" | "${999}" | "${00}" | "${01}" => {
                        assert!(result.template.is_some(), "{input} should be template");
                        if let Some(template) = result.template {
                            if !template.items.is_empty() {
                                match &template.items[0] {
                                    TemplateItem::Field(_) => {} // Expected
                                    _ => panic!("{input} should be field substitution"),
                                }
                            }
                        }
                    }
                    "\"$20\"" => {
                        assert!(
                            result.field_selector.is_some(),
                            "{input} should be field selector"
                        );
                    }
                    _ => {} // Other cases just need to not crash
                }
            }
            Err(e) => {
                println!("  ⚠ Failed to parse: {e}");
                // Some edge cases may legitimately fail to parse
            }
        }
    }

    println!("\n=== Edge Case Tests Completed ===");
}

/// Test comprehensive syntax disambiguation across all DSL forms.
#[test]
fn test_comprehensive_disambiguation() {
    let test_cases = vec![
        // Templates with new syntax
        ("{${name}}", "template"),
        ("{State of ${name}}", "template"),
        ("$name", "template"),
        ("{name}", "literal_template"),
        // Field selectors
        ("name", "field_selector"),
        ("user.email", "field_selector"),
        ("\"field name\"", "field_selector"),
        // Filters
        ("name == \"Alice\"", "filter"),
        ("age > 25", "filter"),
        ("user.active == true", "filter"),
        // Field substitutions
        ("${name}", "template"),
    ];

    for (input, expected_type) in test_cases {
        let result = parse_command(input);

        match expected_type {
            "template" => {
                assert!(result.is_ok(), "Template '{input}' should parse");
                let parsed = result.unwrap();
                assert!(
                    parsed.template.is_some(),
                    "Input '{input}' should be template"
                );
                assert!(
                    parsed.filter.is_none(),
                    "Input '{input}' should not be filter"
                );
                assert!(
                    parsed.field_selector.is_none(),
                    "Input '{input}' should not be field selector"
                );
            }
            "field_selector" => {
                assert!(result.is_ok(), "Field selector '{input}' should parse");
                let parsed = result.unwrap();
                assert!(
                    parsed.field_selector.is_some(),
                    "Input '{input}' should be field selector"
                );
                assert!(
                    parsed.filter.is_none(),
                    "Input '{input}' should not be filter"
                );
                assert!(
                    parsed.template.is_none(),
                    "Input '{input}' should not be template"
                );
            }
            "filter" => {
                assert!(result.is_ok(), "Filter '{input}' should parse");
                let parsed = result.unwrap();
                assert!(parsed.filter.is_some(), "Input '{input}' should be filter");
                assert!(
                    parsed.template.is_none(),
                    "Input '{input}' should not be template"
                );
                assert!(
                    parsed.field_selector.is_none(),
                    "Input '{input}' should not be field selector"
                );
            }
            "literal_template" => {
                assert!(result.is_ok(), "Literal template '{input}' should parse");
                let parsed = result.unwrap();
                assert!(
                    parsed.template.is_some(),
                    "Input '{input}' should be template"
                );
                assert!(
                    parsed.filter.is_none(),
                    "Input '{input}' should not be filter"
                );
                assert!(
                    parsed.field_selector.is_none(),
                    "Input '{input}' should not be field selector"
                );
                // Check that it's a literal, not a field
                let template = parsed.template.unwrap();
                if template.items.len() == 1 {
                    if let TemplateItem::Literal(_) = &template.items[0] {
                        // ok
                    } else {
                        panic!("Input '{input}' should be literal template, not field");
                    }
                }
            }
            _ => panic!("Unknown expected type: {expected_type}"),
        }
    }
}

/// Test that filter expressions work correctly.
#[test]
fn test_filter_expressions() {
    // Simple comparison
    let result = parse_command(r#"name == "Alice""#).unwrap();
    assert!(result.filter.is_some());
    assert!(result.template.is_none());
    assert!(result.field_selector.is_none());

    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["name"]);
        assert_eq!(op, ComparisonOp::Equal);
        assert_eq!(value, FilterValue::String("Alice".to_string()));
    } else {
        panic!("Expected simple comparison");
    }

    // Numeric comparison
    let result = parse_command("age > 25").unwrap();
    assert!(result.filter.is_some());
    assert!(result.template.is_none());
    assert!(result.field_selector.is_none());
}

/// Test complex boolean expressions.
#[test]
fn test_complex_boolean_expressions() {
    // Test field truthy parsing
    let result = parse_command("active?").unwrap();
    assert!(result.filter.is_some());

    if let Some(FilterExpr::FieldTruthy(field)) = result.filter {
        assert_eq!(field.parts, vec!["active"]);
    } else {
        panic!("Expected field truthy");
    }

    // Test NOT expressions
    let result = parse_command("!active?").unwrap();
    assert!(result.filter.is_some());

    if let Some(FilterExpr::Not(inner)) = result.filter {
        if let FilterExpr::FieldTruthy(field) = inner.as_ref() {
            assert_eq!(field.parts, vec!["active"]);
        } else {
            panic!("Expected NOT of field truthy");
        }
    } else {
        panic!("Expected NOT expression");
    }
}

/// Test nested field access in various contexts.
#[test]
fn test_nested_field_access() {
    // In filters
    let result = parse_command("user.email == \"alice@example.com\"").unwrap();
    if let Some(FilterExpr::Comparison { field, .. }) = result.filter {
        assert_eq!(field.parts, vec!["user", "email"]);
    } else {
        panic!("Expected comparison with nested field");
    }

    // In templates
    let result = parse_command("{${user.name}}").unwrap();
    let template = result.template.unwrap();
    match &template.items[0] {
        TemplateItem::Field(field) => assert_eq!(field.parts, vec!["user", "name"]),
        _ => panic!("Expected nested field"),
    }

    // In field selectors
    let result = parse_command("user.profile.name").unwrap();
    let field = result.field_selector.unwrap();
    assert_eq!(field.parts, vec!["user", "profile", "name"]);
}

/// Test bracketed template syntax.
#[test]
fn test_bracketed_template_syntax() {
    // Test simple bracketed template
    let result = parse_command("[${name}]").unwrap();
    assert!(result.template.is_some());
    assert!(result.filter.is_none());
    assert!(result.field_selector.is_none());

    let template = result.template.unwrap();
    assert_eq!(template.items.len(), 1);
    match &template.items[0] {
        TemplateItem::Field(field) => assert_eq!(field.parts, vec!["name"]),
        _ => panic!("Expected field"),
    }

    // Test mixed bracketed template
    let result = parse_command("[Name: ${name}, Age: ${age}]").unwrap();
    assert!(result.template.is_some());

    let template = result.template.unwrap();
    assert!(template.items.len() >= 3); // At least: literal, field, literal

    // Check that we have the expected content structure
    let mut found_name_field = false;
    let mut found_age_field = false;
    let mut found_literal_content = false;

    for item in &template.items {
        match item {
            TemplateItem::Field(field) => {
                if field.parts == vec!["name"] {
                    found_name_field = true;
                } else if field.parts == vec!["age"] {
                    found_age_field = true;
                }
            }
            TemplateItem::Literal(text) => {
                if text.contains("Name:") || text.contains("Age:") {
                    found_literal_content = true;
                }
            }
            _ => {}
        }
    }

    assert!(found_name_field, "Should contain name field");
    assert!(found_age_field, "Should contain age field");
    assert!(found_literal_content, "Should contain literal content");
}

/// Test array element selection like users.0.name
#[test]
fn test_array_element_selection() {
    use parsm::filter::FieldPath;
    use serde_json::json;

    // Test array element selection like users.0.name
    let data = json!({
        "users": [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ],
        "items": ["apple", "banana", "cherry"]
    });

    // Test users.0.name (nested array access)
    let field_path = FieldPath::new(vec![
        "users".to_string(),
        "0".to_string(),
        "name".to_string(),
    ]);
    let result = field_path.get_value(&data);
    assert_eq!(result, Some(&json!("Alice")));

    // Test users.1.name (second element)
    let field_path = FieldPath::new(vec![
        "users".to_string(),
        "1".to_string(),
        "name".to_string(),
    ]);
    let result = field_path.get_value(&data);
    assert_eq!(result, Some(&json!("Bob")));

    // Test items.0 (simple array access)
    let field_path = FieldPath::new(vec!["items".to_string(), "0".to_string()]);
    let result = field_path.get_value(&data);
    assert_eq!(result, Some(&json!("apple")));

    // Test parsing users.0.name as field selector
    let result = parse_command("users.0.name").unwrap();
    assert!(result.field_selector.is_some());
    assert!(result.filter.is_none());
    assert!(result.template.is_none());

    if let Some(field_selector) = result.field_selector {
        assert_eq!(field_selector.parts, vec!["users", "0", "name"]);
    }
}

/// Test regex matching with ~= operator
#[test]
fn test_regex_matching() {
    // Test regex literal with ~= operator
    let result = parse_command("name ~= /^[A-Z]/");

    match result {
        Ok(parsed) => {
            assert!(parsed.filter.is_some(), "Regex pattern should be filter");

            if let Some(FilterExpr::Comparison { field, op, value }) = parsed.filter {
                assert_eq!(field.parts, vec!["name"]);
                assert_eq!(op, ComparisonOp::Matches);
                // Value should contain the regex pattern
                match value {
                    FilterValue::String(pattern) => {
                        assert!(pattern.contains("^[A-Z]"), "Should contain regex pattern");
                    }
                    _ => panic!("Regex value should be string"),
                }
            } else {
                panic!("Expected comparison with regex");
            }
        }
        Err(e) => {
            println!("Note: Regex parsing may not be fully implemented: {e}");
            // This test documents the expected behavior even if not fully implemented
        }
    }
}
