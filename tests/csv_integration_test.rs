mod common;

use common::command;

#[test]
fn test_csv_field_selection_by_header() {
    // Test field selection by header name on CSV with headers
    let input = "name,age,occupation\nTom,45,engineer\nAlice,30,doctor";

    let mut cmd = command();
    cmd.arg("name");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should extract names from the CSV
    assert!(stdout.contains("Tom"));
    assert!(stdout.contains("Alice"));
    assert!(!stdout.contains("45")); // Should not contain ages
}

#[test]
fn test_csv_field_selection_by_index() {
    // Test field selection by index on CSV without headers
    let input = "Tom,45,engineer\nAlice,30,doctor";

    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should extract first field (names)
    assert!(stdout.contains("Tom"));
    assert!(stdout.contains("Alice"));
    assert!(!stdout.contains("45")); // Should not contain ages
}

#[test]
fn test_csv_header_detection() {
    // Test that headers are correctly detected and skipped in output
    let input = "name,age,occupation\nTom,45,engineer\nAlice,30,doctor";

    let mut cmd = command();
    cmd.arg("name");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain the header word "name" in output
    assert!(!stdout.contains("name"));
    // Should contain actual data
    assert!(stdout.contains("Tom"));
    assert!(stdout.contains("Alice"));
}

#[test]
fn test_csv_no_header_detection() {
    // Test CSV without headers - should not skip first row
    let input = "Tom,45,engineer\nAlice,30,doctor";

    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain both names (no header to skip)
    assert!(stdout.contains("Tom"));
    assert!(stdout.contains("Alice"));

    // Count lines to ensure both rows are processed
    let line_count = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert_eq!(line_count, 2);
}

#[test]
fn test_csv_template_with_headers() {
    // Test template rendering with header-based field access
    let input = "name,age,occupation\nTom,45,engineer\nAlice,30,doctor";

    let mut cmd = command();
    cmd.arg("[$name is $age years old]");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should render template with data
    assert!(stdout.contains("Tom is 45 years old"));
    assert!(stdout.contains("Alice is 30 years old"));
}

#[test]
fn test_csv_filter_with_headers() {
    // Test filtering with header-based field access
    let input = "name,age,occupation\nTom,45,engineer\nAlice,30,doctor\nBob,35,engineer";

    let mut cmd = command();
    cmd.arg("occupation == \"engineer\" {$name}");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should only contain engineers
    assert!(stdout.contains("Tom"));
    assert!(stdout.contains("Bob"));
    assert!(!stdout.contains("Alice")); // Doctor should be filtered out
}

#[test]
fn test_csv_multiple_field_selection() {
    // Test selecting different fields by index
    let input = "Alice,30,Engineer\nBob,25,Designer";

    let mut cmd = command();
    cmd.arg("field_2");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Engineer"));
    assert!(stdout.contains("Designer"));
}

#[test]
fn test_csv_numeric_filtering() {
    // Test filtering CSV data with numeric comparisons
    let input = "Alice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager\nDana,22,Intern";

    let mut cmd = command();
    cmd.arg("field_1 > 27");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should match Alice (30) and Charlie (35)
    assert!(stdout.contains("Alice"));
    assert!(stdout.contains("Charlie"));
    assert!(!stdout.contains("Bob"));
    assert!(!stdout.contains("Dana"));
}

#[test]
fn test_csv_string_filtering() {
    // Test filtering CSV data with string operations
    let input = "Alice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager\nEva,28,Engineer";

    let mut cmd = command();
    cmd.arg("field_2 == \"Engineer\"");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should match Alice and Eva (both Engineers)
    assert!(stdout.contains("Alice"));
    assert!(stdout.contains("Eva"));
    assert!(!stdout.contains("Bob"));
    assert!(!stdout.contains("Charlie"));
}

#[test]
fn test_csv_template_replacement_indexed() {
    // Test template replacement with indexed CSV fields
    let input = "Alice,30,Engineer\nBob,25,Designer";

    let mut cmd = command();
    cmd.arg("field_1 > 20 {Name: $field_0, Age: $field_1, Job: $field_2}");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Name: Alice, Age: 30, Job: Engineer"));
    assert!(stdout.contains("Name: Bob, Age: 25, Job: Designer"));
}

#[test]
fn test_csv_original_input_template() {
    // Test ${0} (original input) in templates
    let input = "Alice,30,Engineer";

    let mut cmd = command();
    cmd.arg("field_1 > 25 {Person: ${field_0} | Original: ${0}}");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(stdout.trim(), "Person: Alice | Original: Alice,30,Engineer");
}

#[test]
fn test_csv_complex_boolean_logic() {
    // Test complex boolean logic with CSV data
    let input = "Alice,30,Engineer,true\nBob,22,Designer,false\nCharlie,35,Manager,false\nDana,24,Admin,true";

    let mut cmd = command();
    cmd.arg("field_1 > 25 && field_3? {Found: $field_0 ($field_2)}");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should only match Alice (age > 25 && active == true)
    assert!(stdout.contains("Found: Alice"));
    assert!(!stdout.contains("Bob"));
    assert!(!stdout.contains("Charlie"));
    assert!(!stdout.contains("Dana"));
}

#[test]
fn test_csv_nonexistent_field() {
    // Test accessing non-existent fields
    let input = "Alice,30,Engineer";

    let mut cmd = command();
    cmd.arg("field_5");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Non-existent field should return empty or be handled gracefully
    assert_eq!(stdout.trim(), "");
}

#[test]
fn test_csv_quoted_fields() {
    // Test CSV with quoted fields containing commas
    let input = r#""Smith, John",35,"Senior Engineer, Tech Lead""#;

    let mut cmd = command();
    cmd.arg("field_1");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(stdout.trim(), "35");
}

#[test]
fn test_csv_empty_fields() {
    // Test CSV with empty fields
    let input = "Alice,,Engineer\n,25,Designer\nCharlie,35,\n";

    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Alice");
    assert_eq!(lines[1], ""); // Empty field
    assert_eq!(lines[2], "Charlie");
}

#[test]
fn test_csv_numeric_comparisons() {
    // Test various numeric comparisons
    let test_cases = vec![
        ("Alice,30,5000", "field_1 > 25", true),
        ("Bob,22,3000", "field_1 < 25", true),
        ("Charlie,30,5000", "field_2 >= 5000", true),
        ("Dana,28,4500", "field_2 <= 4000", false),
        ("Eve,35,6000", "field_1 == 35", true),
        ("Frank,25,3500", "field_1 != 30", true),
    ];

    for (input, filter, should_match) in test_cases {
        let mut cmd = command();
        cmd.arg(format!("{filter} {{Match: $field_0}}"));
        let output = common::run(cmd, input);

        assert!(output.status.success(), "parsm failed for input: {input}");
        let stdout = String::from_utf8_lossy(&output.stdout);

        if should_match {
            assert!(
                stdout.contains("Match:"),
                "Expected match for: {input} with filter: {filter}"
            );
        } else {
            assert_eq!(
                stdout.trim(),
                "",
                "Expected no match for: {input} with filter: {filter}"
            );
        }
    }
}

#[test]
fn test_csv_different_row_lengths() {
    // Test CSV rows with different number of fields
    let input = "Alice,30\nBob,25,Designer,Manager\nCharlie,35,Engineer";

    let mut cmd = command();
    cmd.arg("field_1");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "30");
    assert_eq!(lines[1], "25");
    assert_eq!(lines[2], "35");
}

#[test]
fn test_csv_single_column() {
    // Test CSV with only one column
    let input = "Alice\nBob\nCharlie";

    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, input);

    // This might not be detected as CSV, but if it is, should work
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            let lines: Vec<&str> = stdout.trim().split('\n').collect();
            assert!(lines.len() <= 3); // May or may not be detected as CSV
        }
    }
}

#[test]
fn test_csv_malformed_input() {
    // Test with malformed CSV (unbalanced quotes)
    let input = r#"Alice,30,"Engineer
Bob,25,Designer"#;

    let mut cmd = command();
    cmd.arg("field_0 == \"Bob\"");
    let output = common::run(cmd, input);

    // Should handle malformed CSV gracefully without crashing
    assert!(
        output.status.success(),
        "parsm should handle malformed CSV gracefully"
    );
}

#[test]
fn test_csv_braced_field_syntax() {
    // Test ${field_N} syntax in templates
    let input = "Alice,30,Engineer";

    let mut cmd = command();
    cmd.arg("field_1 > 25 {Name: ${field_0}, Age: ${field_1}, Job: ${field_2}}");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(stdout.trim(), "Name: Alice, Age: 30, Job: Engineer");
}

#[test]
fn test_csv_headers_all_data_rows() {
    // Test that with headers, all data rows (not header) are processed
    let input = "Name,Age,Job\nAlice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager";

    let mut cmd = command();
    cmd.arg("name");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain the header word "Name"
    assert!(!stdout.contains("Name"));
    // Should contain all data rows
    assert!(stdout.contains("Alice"));
    assert!(stdout.contains("Bob"));
    assert!(stdout.contains("Charlie"));

    let line_count = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert_eq!(line_count, 3); // Only data rows, not header
}

/// Test CSV forced format detection with --csv flag
#[test]
fn test_csv_forced_format() {
    let test_cases = vec![
        // Comma-separated data that might be detected as other formats, but force CSV parsing
        (
            "name,age,occupation\nAlice,30,Engineer",
            r#""name""#,
            "Alice",
        ),
        ("name,age,occupation\nAlice,30,Engineer", r#""age""#, "30"),
        ("Alice,30,Engineer", r#""field_0""#, "Alice"),
        ("Alice,30,Engineer", r#""field_1""#, "30"),
        ("Alice,30,Engineer", r#""field_2""#, "Engineer"),
        // Comma-separated with quotes, ensure it's parsed as CSV
        ("\"Smith, John\",30,Engineer", r#""field_0""#, "Smith, John"),
        ("\"Smith, John\",30,Engineer", r#""field_1""#, "30"),
        // Test template with forced CSV
        (
            "Alice,30,Engineer",
            r#"{Name: ${1}, Age: ${2}}"#,
            "Name: Alice, Age: 30",
        ),
        (
            "Alice,30,Engineer",
            r#"{${field_0} is ${field_1} years old}"#,
            "Alice is 30 years old",
        ),
    ];

    for (input, expression, expected) in test_cases {
        let mut cmd = command();
        cmd.arg("--csv").arg(expression);
        let output = common::run(cmd, input);

        assert!(
            output.status.success(),
            "CSV forced format failed for input '{}' with expression '{}': {:?}",
            input,
            expression,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();
        assert_eq!(
            result, expected,
            "Failed for CSV forced input '{input}' with expression '{expression}'",
        );
    }
}

/// Test CSV forced format filtering with --csv flag
#[test]
fn test_csv_forced_format_filtering() {
    let test_cases = vec![
        ("Alice,30,Engineer", "field_1 > \"25\"", true),
        ("Bob,20,Student", "field_1 > \"25\"", false),
        ("Alice,30,Engineer", r#"field_0 == "Alice""#, true),
        ("Bob,20,Student", r#"field_0 == "Alice""#, false),
        ("Alice,30,Engineer", r#"field_2 *= "Eng""#, true),
        ("Bob,20,Student", r#"field_2 *= "Eng""#, false),
    ];

    for (input, filter, should_match) in test_cases {
        let mut cmd = command();
        cmd.arg("--csv").arg(filter).arg(r#"{match}"#);
        let output = common::run(cmd, input);

        assert!(
            output.status.success(),
            "CSV forced format filtering failed for input '{}' with filter '{}': {:?}",
            input,
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();

        if should_match {
            assert_eq!(
                result, "match",
                "Expected match for CSV forced filter '{filter}' with input '{input}'",
            );
        } else {
            assert_eq!(
                result, "",
                "Expected empty output for CSV forced filter '{filter}' with input '{input}'",
            );
        }
    }
}

/// Test multiline CSV output with consistent handling
#[test]
fn test_csv_multiline_output() {
    // Test with a multiline CSV file that has headers
    let input = "name,age,occupation\nAlice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager";

    let cmd = command();
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            r#"{"name":"Alice","age":"30","occupation":"Engineer"}"#,
            "\n",
            r#"{"name":"Bob","age":"25","occupation":"Designer"}"#,
            "\n",
            r#"{"name":"Charlie","age":"35","occupation":"Manager"}"#,
            "\n",
        )
    );

    // Test filtering functionality on multiline CSV
    let mut cmd = command();
    cmd.arg("age > 27"); // Filter rows where age > 27
    let filter_output = common::run(cmd, input);

    assert!(
        filter_output.status.success(),
        "parsm filter failed: {filter_output:?}"
    );
    let filter_stdout = common::stdout_of(&filter_output);

    // Should only contain rows with age > 27 (Alice and Charlie)
    assert!(filter_stdout.contains("Alice,30,Engineer"));
    assert!(filter_stdout.contains("Charlie,35,Manager"));
    assert!(!filter_stdout.contains("Bob,25,Designer"));

    // Header-based filtering with --csv flag needs field_N syntax instead of
    // header names.
    let mut cmd = command();
    cmd.arg("--csv").arg("field_1 > 27");
    let header_filter_output = common::run(cmd, input);

    assert!(
        header_filter_output.status.success(),
        "parsm header filter failed: {header_filter_output:?}"
    );
    let header_filter_stdout = common::stdout_of(&header_filter_output);

    // Should only contain rows with age > 27 (Alice and Charlie)
    assert!(header_filter_stdout.contains("Alice,30,Engineer"));
    assert!(header_filter_stdout.contains("Charlie,35,Manager"));
    assert!(!header_filter_stdout.contains("Bob,25,Designer"));
}

/// Test multiline CSV output with consistent handling
#[test]
fn test_csv_multiline_output_field() {
    // Test with a multiline CSV file that has headers
    let input = "name,age,occupation\nAlice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager";

    let cmd = command();
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            r#"{"name":"Alice","age":"30","occupation":"Engineer"}"#,
            "\n",
            r#"{"name":"Bob","age":"25","occupation":"Designer"}"#,
            "\n",
            r#"{"name":"Charlie","age":"35","occupation":"Manager"}"#,
            "\n",
        )
    );

    // Test filtering functionality on multiline CSV
    let mut cmd = command();
    cmd.arg("field_1 > 27"); // Filter rows where age > 27
    let filter_output = common::run(cmd, input);

    assert!(
        filter_output.status.success(),
        "parsm filter failed: {filter_output:?}"
    );
    let filter_stdout = common::stdout_of(&filter_output);

    // Should only contain rows with age > 27 (Alice and Charlie)
    assert!(filter_stdout.contains("Alice,30,Engineer"));
    assert!(filter_stdout.contains("Charlie,35,Manager"));
    assert!(!filter_stdout.contains("Bob,25,Designer"));

    // Header-based filtering with --csv flag needs field_N syntax instead of
    // header names.
    let mut cmd = command();
    cmd.arg("--csv").arg("field_1 > 27");
    let header_filter_output = common::run(cmd, input);

    assert!(
        header_filter_output.status.success(),
        "parsm header filter failed: {header_filter_output:?}"
    );
    let header_filter_stdout = common::stdout_of(&header_filter_output);

    // Should only contain rows with age > 27 (Alice and Charlie)
    assert!(header_filter_stdout.contains("Alice,30,Engineer"));
    assert!(header_filter_stdout.contains("Charlie,35,Manager"));
    assert!(!header_filter_stdout.contains("Bob,25,Designer"));
}

/// Regression test for multiline CSV output functionality
///
/// This test verifies that the multiline CSV output functionality works correctly,
/// focusing on the default behavior of outputting the original input and ensuring
/// that filtering works properly.
#[test]
fn test_csv_output_regression() {
    // Create a simple CSV file without complex quoting
    let input = "name,age,active\nAlice,30,true\nBob,25,false\nCharlie,35,true";

    let mut cmd = command();
    cmd.arg("--csv");
    let output = common::run(cmd, input);

    assert!(
        output.status.success(),
        "parsm default output failed: {output:?}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            r#"{"name":"Alice","age":"30","active":"true"}"#,
            "\n",
            r#"{"name":"Bob","age":"25","active":"false"}"#,
            "\n",
            r#"{"name":"Charlie","age":"35","active":"true"}"#,
            "\n",
        )
    );

    // Test filtering by field index
    let mut cmd = command();
    cmd.arg("--csv").arg("field_2 == \"true\"");
    let filter_output = common::run(cmd, input);

    assert!(
        filter_output.status.success(),
        "parsm filter failed: {filter_output:?}"
    );
    let filter_stdout = common::stdout_of(&filter_output);

    // Should only include rows with active=true
    assert!(filter_stdout.contains("Alice,30,true"));
    assert!(filter_stdout.contains("Charlie,35,true"));
    assert!(!filter_stdout.contains("Bob,25,false"));

    // Test field selection
    let mut cmd = command();
    cmd.arg("--csv").arg("field_0");
    let field_output = common::run(cmd, input);

    assert!(
        field_output.status.success(),
        "parsm field selection failed: {field_output:?}"
    );
    let field_stdout = common::stdout_of(&field_output);

    // Should extract just the names
    assert_eq!(
        field_stdout.trim().lines().collect::<Vec<&str>>().join(","),
        "Alice,Bob,Charlie"
    );
}

#[test]
fn forced_csv_converts_a_line_without_commas() {
    let mut cmd = command();
    cmd.arg("--csv");
    let output = common::run(cmd, "a b");
    assert_eq!(common::stdout_of(&output), "[\"a b\"]\n");
    assert!(output.status.success());
}

#[test]
fn csv_without_header_converts_to_field_arrays() {
    let output = common::run(command(), "1,2,3");
    assert_eq!(common::stdout_of(&output), "[\"1\",\"2\",\"3\"]\n");
    assert!(output.status.success());
}

#[test]
fn csv_undecodable_later_line_warns_and_is_skipped() {
    let mut cmd = command();
    cmd.arg("name");
    let output = common::run(cmd, b"name,age\nTom,45\n\xff,1\nAnn,30\n");
    assert_eq!(common::stdout_of(&output), "Tom\nAnn\n");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Warning: failed to parse line 3: invalid UTF-8\n"
    );
    assert!(output.status.success());
}

#[test]
fn csv_rows_after_the_header_sample_stream() {
    let mut cmd = command();
    cmd.arg("name");
    let mut session = common::Session::start(cmd);
    session.write_line("name,age");
    for index in 1..=6 {
        session.write_line(&format!("user{index},{}", 20 + index));
    }
    let expected: Vec<String> = (1..=6).map(|index| format!("user{index}")).collect();
    assert_eq!(session.read_lines(6), expected);
    session.write_line("user7,27");
    assert_eq!(session.read_lines(1), vec!["user7"]);
    let (rest, status) = session.finish();
    assert!(rest.is_empty(), "rest: {rest:?}");
    assert!(status.success());
}

#[test]
fn loose_comma_prose_is_read_as_text_words() {
    let mut cmd = command();
    cmd.arg("[${word_1}]");
    let output = common::run(cmd, "Hello, world");
    assert_eq!(common::stdout_of(&output), "world\n");
    assert!(output.status.success());
}

#[test]
fn loose_comma_prose_converts_as_a_word_array() {
    let output = common::run(command(), "Hello, world");
    assert_eq!(common::stdout_of(&output), "[\"Hello,\",\"world\"]\n");
    assert!(output.status.success());
}

#[test]
fn tight_comma_line_is_csv_with_positional_fields() {
    let mut cmd = command();
    cmd.arg(r#"field_1 > "25" [${1} (${2})]"#);
    let output = common::run(cmd, "Alice,30,Engineer");
    assert_eq!(common::stdout_of(&output), "Alice (30)\n");
    assert!(output.status.success());
}

#[test]
fn quoted_comma_keeps_the_line_tight_csv() {
    let mut cmd = command();
    cmd.arg("[${field_0}]");
    let output = common::run(cmd, "\"Smith, John\",30");
    assert_eq!(common::stdout_of(&output), "Smith, John\n");
    assert!(output.status.success());
}

#[test]
fn loose_comma_with_a_matching_next_line_is_csv() {
    let mut cmd = command();
    cmd.arg("[${field_0}]");
    let output = common::run(cmd, "x, y\n1, 2");
    assert_eq!(common::stdout_of(&output), "1\n");
    assert!(output.status.success());
}

#[test]
fn header_keys_keep_case_and_a_lower_case_alias() {
    let mut cmd = command();
    cmd.arg("[${Name}/${name}]");
    let output = common::run(cmd, "Name,Age\nAlice,30");
    assert_eq!(common::stdout_of(&output), "Alice/Alice\n");
    assert!(output.status.success());
}

#[test]
fn ragged_rows_keep_every_field_in_convert_mode() {
    let output = common::run(command(), "name,age\nAlice,30\nBob,40,extra\nCarol");
    assert_eq!(
        common::stdout_of(&output),
        concat!(
            r#"{"name":"Alice","age":"30"}"#,
            "\n",
            r#"{"name":"Bob","age":"40","field_2":"extra"}"#,
            "\n",
            r#"{"name":"Carol"}"#,
            "\n",
        )
    );
    assert!(output.status.success());
}

#[test]
fn runaway_quote_fails_its_line_and_resumes_after_it() {
    let mut cmd = command();
    cmd.arg("[${a}]");
    let mut input = String::from("a,b\n\"bad,1\n");
    for n in 1..=100 {
        input.push_str(&format!("{n},{n}\n"));
    }
    let output = common::run(cmd, input);
    let expected: String = (1..=100).map(|n| format!("{n}\n")).collect();
    assert_eq!(common::stdout_of(&output), expected);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Warning: failed to parse line 2: quoted field spans more than 64 lines\n"
    );
    assert!(output.status.success());
}

#[test]
fn runaway_quote_recovery_streams_before_stdin_closes() {
    let mut cmd = command();
    cmd.arg("[${a}]");
    let mut session = common::Session::start(cmd);
    session.write_line("a,b");
    session.write_line("\"bad,1");
    // The 64-line bound trips on its own once these lines are fed, with no
    // further input needed — proving recovery does not wait for EOF.
    for n in 1..=63 {
        session.write_line(&format!("{n},{n}"));
    }
    assert_eq!(session.read_lines(1), vec!["1"]);
    session.write_line("64,64");
    let (rest, status) = session.finish();
    assert!(rest.contains(&"64".to_string()), "rest: {rest:?}");
    assert!(status.success());
}

#[test]
fn quoted_field_within_the_bound_stays_one_record() {
    let output = common::run(command(), "a,b\n\"x\ny\",2");
    assert_eq!(
        common::stdout_of(&output),
        "{\"a\":\"x\\ny\",\"b\":\"2\"}\n"
    );
    assert!(output.status.success());
}

// Ported from the retired Python integration harness; each asserts the
// same input, arguments and expected stdout as its original case (see
// the batch b6 REPORT's mapping table).

#[test]
fn ported_csv_field_select() {
    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, "Alice,30,Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice");
}

#[test]
fn ported_csv_indexed_select() {
    let mut cmd = command();
    cmd.arg("field_2");
    let output = common::run(cmd, "Alice,30,Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Engineer");
}

#[test]
fn ported_csv_filter_string() {
    let mut cmd = command();
    cmd.arg(r#"field_1 > "25""#);
    let output = common::run(cmd, "Alice,30,Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice,30,Engineer");
}

#[test]
fn ported_csv_template_simple() {
    let mut cmd = command();
    cmd.arg("{${field_0} works as ${field_2}}");
    let output = common::run(cmd, "Alice,30,Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice works as Engineer");
}

#[test]
fn ported_csv_template_indexed() {
    let mut cmd = command();
    cmd.arg("{${1} - ${2} - ${3}}");
    let output = common::run(cmd, "Alice,30,Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice - 30 - Engineer");
}

#[test]
fn ported_csv_empty_field() {
    let mut cmd = command();
    cmd.arg("field_1");
    let output = common::run(cmd, "Alice,,Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "");
}

#[test]
fn ported_csv_multiline() {
    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, "Alice,30\nBob,25");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice\nBob");
}

#[test]
fn ported_csv_header_field_select() {
    let mut cmd = command();
    cmd.arg("name");
    let output = common::run(cmd, "name,age,occupation\nTom,45,engineer\nAlice,30,doctor");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Tom\nAlice");
}

#[test]
fn ported_csv_header_detection() {
    let mut cmd = command();
    cmd.arg("age");
    let output = common::run(cmd, "name,age,occupation\nTom,45,engineer\nAlice,30,doctor");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "45\n30");
}

#[test]
fn ported_csv_no_header_detection() {
    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, "Tom,45,engineer\nAlice,30,doctor");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Tom\nAlice");
}

#[test]
fn ported_csv_template_headers() {
    let mut cmd = command();
    cmd.arg("{${name} is ${age} years old}");
    let output = common::run(cmd, "name,age,occupation\nTom,45,engineer\nAlice,30,doctor");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(
        common::stdout_of(&output).trim(),
        "Tom is 45 years old\nAlice is 30 years old"
    );
}

#[test]
fn ported_csv_filter_headers() {
    let mut cmd = command();
    cmd.arg(r#"occupation == "engineer" {$name}"#);
    let output = common::run(
        cmd,
        "name,age,occupation\nTom,45,engineer\nAlice,30,doctor\nBob,35,engineer",
    );
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Tom\nBob");
}

#[test]
fn ported_csv_mixed_header_patterns() {
    let mut cmd = command();
    cmd.arg("firstname");
    let output = common::run(
        cmd,
        "user_id,firstName,Last_Name,emailAddress\n1,John,Doe,john@example.com\n2,Jane,Smith,jane@example.com",
    );
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "John\nJane");
}

#[test]
fn ported_detect_csv() {
    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, "col1,col2,col3");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "col1");
}

#[test]
fn ported_special_chars() {
    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, r#"test@domain.com,123,"value with spaces""#);
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "test@domain.com");
}

#[test]
fn ported_stream_csv() {
    let mut cmd = command();
    cmd.arg("field_0");
    let output = common::run(cmd, "Alice,30\nBob,25\nCharlie,35");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice\nBob\nCharlie");
}

#[test]
fn ported_truthy_csv_present() {
    let mut cmd = command();
    cmd.arg("field_1?");
    let output = common::run(cmd, "Alice,30,Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice,30,Engineer");
}

#[test]
fn ported_truthy_csv_empty() {
    let mut cmd = command();
    cmd.arg("field_1?");
    let output = common::run(cmd, "Alice,,Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "");
}

#[test]
fn ported_explicit_csv() {
    let mut cmd = command();
    cmd.args(["--csv", "field_0"]);
    let output = common::run(cmd, "Alice,30,Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice");
}

#[test]
fn ported_json_as_csv() {
    let mut cmd = command();
    cmd.args(["--csv", "field_0"]);
    let output = common::run(cmd, r#"{"name": "Alice", "age": 30}"#);
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), r#"{"name": "Alice""#);
}

#[test]
fn ported_explicit_csv_filter() {
    let mut cmd = command();
    cmd.args(["--csv", r#"field_1 > "27""#]);
    let output = common::run(cmd, "Alice,30,Engineer\nBob,25,Designer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice,30,Engineer");
}
/// Kitchen sink: positional field access, a case-insensitive regex, `&&`/
/// `||`/`!field?`, and a template conditional plus `${0}`, combined in one
/// expression and asserted against exact output.
#[test]
fn csv_kitchen_sink() {
    let input = "Alice,30,engineer,alice@EXAMPLE.com,true,false";
    let mut cmd = command();
    cmd.args([
        "--csv",
        r#"(field_3 ~= /example\.com/i || field_2 == "engineer") && !field_5? {Role: ${field_2} - Active: ${field_4?yes:no} - Source: ${0}}"#,
    ]);
    let output = common::run(cmd, input);
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(
        common::stdout_of(&output).trim_end_matches('\n'),
        format!("Role: engineer - Active: yes - Source: {input}")
    );
}
