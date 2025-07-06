#[cfg(test)]
mod ambiguous_regression_tests {
    use parsm::parse_command;

    /// Test that ambiguous expressions are properly rejected
    #[test]
    fn test_ambiguous_expressions_rejected() {
        let ambiguous_expressions = vec![
            // Bare field names in boolean expressions (should require ? for truthy)
            "name && age",
            "user || admin",
            "active && !disabled",
            "status || role",
            // Mixed bare fields with templates (ambiguous intent)
            r#"name && age [${name}]"#,
            r#"user || admin {${user}}"#,
            r#"active && disabled [User: ${name}]"#,
            // Bare fields with comparison operators but missing operands
            "name &&",
            "age ||",
            "status == ",
            // Invalid boolean combinations
            "name age",   // Missing operator
            "name & age", // Wrong operator (single &)
            "name | age", // Wrong operator (single |)
            // Templates with boolean operators outside of brackets/braces
            r#"Hello name && age ${name}"#,
            r#"Status: active || inactive [${status}]"#,
        ];

        println!("\n=== Testing Ambiguous Expression Rejection ===");

        for expr in ambiguous_expressions {
            println!("\nTesting ambiguous expression: '{expr}'");

            match parse_command(expr) {
                Ok(parsed) => {
                    panic!(
                        "Expected rejection but got successful parse for '{expr}': filter={:?}, template={:?}, field_selector={:?}",
                        parsed.filter, parsed.template, parsed.field_selector
                    );
                }
                Err(e) => {
                    println!("✓ Correctly rejected: {e}");

                    // Verify error message mentions the ambiguity
                    let error_msg = e.to_string().to_lowercase();
                    assert!(
                        error_msg.contains("ambiguous")
                            || error_msg.contains("expected")
                            || error_msg.contains("comparison_op")
                            || error_msg.contains("could not parse")
                            || error_msg.contains("invalid"),
                        "Error message should indicate the problem: {e}"
                    );
                }
            }
        }
    }

    /// Test that valid expressions continue to work
    #[test]
    fn test_valid_expressions_accepted() {
        let valid_expressions = vec![
            // Valid truthy expressions
            ("name?", "single truthy"),
            ("name? && age?", "combined truthy"),
            ("user? || admin?", "truthy with OR"),
            ("!(status?)", "negated truthy"),
            // Valid comparison expressions
            (r#"name == "Alice""#, "string comparison"),
            ("age > 25", "numeric comparison"),
            (r#"status != "disabled""#, "not equal"),
            (r#"name == "Alice" && age > 25"#, "combined comparisons"),
            // Valid field selectors
            ("name", "simple field"),
            ("user.email", "nested field"),
            (r#""field name""#, "quoted field"),
            // Valid templates
            (r#"{${name}}"#, "braced template"),
            (r#"[${name}]"#, "bracketed template"),
            ("$name", "simple variable"),
            (r#"{Hello ${name}!}"#, "braced interpolated"),
            (r#"[User: ${name}, Age: ${age}]"#, "bracketed interpolated"),
            // Valid combined filter + template
            (
                r#"name? && age? {${name}}"#,
                "truthy filter with braced template",
            ),
            (
                r#"age > 25 [${name}]"#,
                "comparison filter with bracketed template",
            ),
            (
                r#"status == "active" {Active user: ${name}}"#,
                "comparison with interpolated template",
            ),
        ];

        println!("\n=== Testing Valid Expression Acceptance ===");

        for (expr, description) in valid_expressions {
            println!("\nTesting valid expression ({description}): '{expr}'");

            match parse_command(expr) {
                Ok(parsed) => {
                    println!("✓ Correctly accepted");
                    // Verify at least one component was parsed
                    assert!(
                        parsed.filter.is_some()
                            || parsed.template.is_some()
                            || parsed.field_selector.is_some(),
                        "Expected at least one component to be parsed for '{expr}'"
                    );
                }
                Err(e) => {
                    panic!("Expected acceptance but got error for '{expr}' ({description}): {e}");
                }
            }
        }
    }

    /// Test specific edge cases that were problematic
    #[test]
    fn test_specific_edge_cases() {
        println!("\n=== Testing Specific Edge Cases ===");

        // Test the original problematic case
        let problematic_expr = "name && age [${name}]";
        println!("\nTesting original problematic case: '{problematic_expr}'");
        match parse_command(problematic_expr) {
            Ok(_) => panic!("Should reject '{problematic_expr}' as ambiguous"),
            Err(e) => {
                println!("✓ Correctly rejected: {e}");
                let error_msg = e.to_string().to_lowercase();
                assert!(
                    error_msg.contains("expected") || error_msg.contains("could not parse"),
                    "Error message should mention parsing failure: {e}"
                );
            }
        }

        // Test that the corrected version works
        let corrected_expr = "name? && age? [${name}]";
        println!("\nTesting corrected version: '{corrected_expr}'");
        match parse_command(corrected_expr) {
            Ok(parsed) => {
                println!("✓ Correctly accepted corrected version");
                assert!(parsed.filter.is_some(), "Should have filter");
                assert!(parsed.template.is_some(), "Should have template");
            }
            Err(e) => panic!("Should accept '{corrected_expr}': {e}"),
        }

        // Test bare interpolated text is rejected
        let bare_interpolated = "Hello ${name}";
        println!("\nTesting bare interpolated text: '{bare_interpolated}'");
        match parse_command(bare_interpolated) {
            Ok(_) => panic!("Should reject bare interpolated text '{bare_interpolated}'"),
            Err(e) => {
                println!("✓ Correctly rejected bare interpolated text: {e}");
            }
        }

        // Test bracketed interpolated text is accepted
        let bracketed_interpolated = "[Hello ${name}]";
        println!("\nTesting bracketed interpolated text: '{bracketed_interpolated}'");
        match parse_command(bracketed_interpolated) {
            Ok(parsed) => {
                println!("✓ Correctly accepted bracketed interpolated text");
                assert!(parsed.template.is_some(), "Should have template");
            }
            Err(e) => panic!("Should accept '{bracketed_interpolated}': {e}"),
        }
    }
}
