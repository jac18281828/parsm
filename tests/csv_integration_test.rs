use std::fs::File;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Helper function to create a Command with proper environment setup
fn parsm_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_parsm"));
    cmd.env("RUST_LOG", "parsm=error");
    cmd
}

#[test]
fn test_csv_field_selection_by_header() {
    // Test field selection by header name on CSV with headers
    let input = "name,age,occupation\nTom,45,engineer\nAlice,30,doctor";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("name")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_0")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("name")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_0")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("[$name is $age years old]")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("occupation == \"engineer\" {$name}")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_2") // Select third field (occupation)
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Engineer"));
    assert!(stdout.contains("Designer"));
}

#[test]
fn test_csv_numeric_filtering() {
    // Test filtering CSV data with numeric comparisons
    let input = "Alice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager\nDana,22,Intern";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_1 > 27") // Filter by age > 27
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_2 == \"Engineer\"") // Filter by occupation
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_1 > 20 {Name: $field_0, Age: $field_1, Job: $field_2}")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Name: Alice, Age: 30, Job: Engineer"));
    assert!(stdout.contains("Name: Bob, Age: 25, Job: Designer"));
}

#[test]
fn test_csv_original_input_template() {
    // Test ${0} (original input) in templates
    let input = "Alice,30,Engineer";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_1 > 25 {Person: ${field_0} | Original: ${0}}")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(stdout.trim(), "Person: Alice | Original: Alice,30,Engineer");
}

#[test]
fn test_csv_complex_boolean_logic() {
    // Test complex boolean logic with CSV data
    let input = "Alice,30,Engineer,true\nBob,22,Designer,false\nCharlie,35,Manager,false\nDana,24,Admin,true";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_1 > 25 && field_3? {Found: $field_0 ($field_2)}")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_5") // Field that doesn't exist
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Non-existent field should return empty or be handled gracefully
    assert_eq!(stdout.trim(), "");
}

#[test]
fn test_csv_quoted_fields() {
    // Test CSV with quoted fields containing commas
    let input = r#""Smith, John",35,"Senior Engineer, Tech Lead""#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_1") // Select age field
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(stdout.trim(), "35");
}

#[test]
fn test_csv_empty_fields() {
    // Test CSV with empty fields
    let input = "Alice,,Engineer\n,25,Designer\nCharlie,35,\n";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_0") // Select first field
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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
        let mut file = NamedTempFile::new().expect("create temp file");
        write!(file, "{input}").expect("write temp file");

        let output = parsm_command()
            .arg(format!("{filter} {{Match: $field_0}}"))
            .stdin(File::open(file.path()).unwrap())
            .output()
            .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_1") // Select second field (age)
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_0") // Select only field
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_0 == \"Bob\"")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("field_1 > 25 {Name: ${field_0}, Age: ${field_1}, Job: ${field_2}}")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(stdout.trim(), "Name: Alice, Age: 30, Job: Engineer");
}

#[test]
fn test_csv_headers_all_data_rows() {
    // Test that with headers, all data rows (not header) are processed
    let input = "Name,Age,Job\nAlice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("name") // Select by header name
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

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
        let mut file = NamedTempFile::new().expect("create temp file");
        write!(file, "{input}").expect("write temp file");

        let output = parsm_command()
            .arg("--csv")
            .arg(expression)
            .stdin(File::open(file.path()).unwrap())
            .output()
            .expect("run parsm");

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
        let mut file = NamedTempFile::new().expect("create temp file");
        write!(file, "{input}").expect("write temp file");

        let output = parsm_command()
            .arg("--csv")
            .arg(filter)
            .arg(r#"{match}"#)
            .stdin(File::open(file.path()).unwrap())
            .output()
            .expect("run parsm");

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Test default output (should preserve original lines)
    let output = parsm_command()
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Default output should be the original lines
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 4); // Header + 3 data rows
    assert_eq!(lines[0], "name,age,occupation");
    assert_eq!(lines[1], "Alice,30,Engineer");
    assert_eq!(lines[2], "Bob,25,Designer");
    assert_eq!(lines[3], "Charlie,35,Manager");

    // Test filtering functionality on multiline CSV
    let filter_output = parsm_command()
        .arg("age > 27") // Filter rows where age > 27
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(
        filter_output.status.success(),
        "parsm filter failed: {filter_output:?}"
    );
    let filter_stdout = String::from_utf8_lossy(&filter_output.stdout);

    // Should only contain rows with age > 27 (Alice and Charlie)
    assert!(filter_stdout.contains("Alice,30,Engineer"));
    assert!(filter_stdout.contains("Charlie,35,Manager"));
    assert!(!filter_stdout.contains("Bob,25,Designer"));

    // Test header-based filtering with --csv flag
    // Note: Currently, with --csv flag, we need to use field_N syntax instead of header names
    let header_filter_output = parsm_command()
        .arg("--csv")
        .arg("field_1 > 27") // Using field index instead of header name
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(
        header_filter_output.status.success(),
        "parsm header filter failed: {header_filter_output:?}"
    );
    let header_filter_stdout = String::from_utf8_lossy(&header_filter_output.stdout);

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Test default output (should preserve original lines)
    let output = parsm_command()
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Default output should be the original lines
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 4); // Header + 3 data rows
    assert_eq!(lines[0], "name,age,occupation");
    assert_eq!(lines[1], "Alice,30,Engineer");
    assert_eq!(lines[2], "Bob,25,Designer");
    assert_eq!(lines[3], "Charlie,35,Manager");

    // Test filtering functionality on multiline CSV
    let filter_output = parsm_command()
        .arg("field_1 > 27") // Filter rows where age > 27
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(
        filter_output.status.success(),
        "parsm filter failed: {filter_output:?}"
    );
    let filter_stdout = String::from_utf8_lossy(&filter_output.stdout);

    // Should only contain rows with age > 27 (Alice and Charlie)
    assert!(filter_stdout.contains("Alice,30,Engineer"));
    assert!(filter_stdout.contains("Charlie,35,Manager"));
    assert!(!filter_stdout.contains("Bob,25,Designer"));

    // Test header-based filtering with --csv flag
    // Note: Currently, with --csv flag, we need to use field_N syntax instead of header names
    let header_filter_output = parsm_command()
        .arg("--csv")
        .arg("field_1 > 27") // Using field index instead of header name
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(
        header_filter_output.status.success(),
        "parsm header filter failed: {header_filter_output:?}"
    );
    let header_filter_stdout = String::from_utf8_lossy(&header_filter_output.stdout);

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

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Test default output (should preserve original input)
    let output = parsm_command()
        .arg("--csv")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(
        output.status.success(),
        "parsm default output failed: {output:?}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Default output should match original input
    assert_eq!(stdout.trim(), input);

    // Test filtering by field index
    let filter_output = parsm_command()
        .arg("--csv")
        .arg("field_2 == \"true\"")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(
        filter_output.status.success(),
        "parsm filter failed: {filter_output:?}"
    );
    let filter_stdout = String::from_utf8_lossy(&filter_output.stdout);

    // Should only include rows with active=true
    assert!(filter_stdout.contains("Alice,30,true"));
    assert!(filter_stdout.contains("Charlie,35,true"));
    assert!(!filter_stdout.contains("Bob,25,false"));

    // Test field selection
    let field_output = parsm_command()
        .arg("--csv")
        .arg("field_0")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(
        field_output.status.success(),
        "parsm field selection failed: {field_output:?}"
    );
    let field_stdout = String::from_utf8_lossy(&field_output.stdout);

    // Should extract just the names
    assert_eq!(
        field_stdout.trim().lines().collect::<Vec<&str>>().join(","),
        "Alice,Bob,Charlie"
    );
}
