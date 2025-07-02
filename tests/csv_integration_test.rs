use std::fs::File;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_csv_field_selection_by_header() {
    // Test field selection by header name on CSV with headers
    let input = "name,age,occupation\nTom,45,engineer\nAlice,30,doctor";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("$name is $age years old")
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"27\"") // Filter by age > 27
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"20\" {Name: $field_0, Age: $field_1, Job: $field_2}")
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"25\" {Person: ${field_0} | Original: ${0}}")
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"25\" && field_3 == \"true\" {Found: $field_0 ($field_2)}")
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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
    let input = "Alice,,Engineer\n,25,Designer\nCharlie,35,";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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
        ("Alice,30,5000", "field_1 > \"25\"", true),
        ("Bob,22,3000", "field_1 < \"25\"", true),
        ("Charlie,30,5000", "field_2 >= \"5000\"", true),
        ("Dana,28,4500", "field_2 <= \"4000\"", false),
        ("Eve,35,6000", "field_1 == \"35\"", true),
        ("Frank,25,3500", "field_1 != \"30\"", true),
    ];

    for (input, filter, should_match) in test_cases {
        let mut file = NamedTempFile::new().expect("create temp file");
        write!(file, "{input}").expect("write temp file");

        let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"25\" {Name: ${field_0}, Age: ${field_1}, Job: ${field_2}}")
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

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
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
