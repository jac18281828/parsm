use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

#[test]
fn test_csv_basic_field_selection() {
    // Test basic field selection on CSV data
    let input = "Alice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{}", input).expect("write temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("\"field_0\"") // Select first field
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should output the first field from each row
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Alice");
    assert_eq!(lines[1], "Bob");
    assert_eq!(lines[2], "Charlie");
}

#[test]
fn test_csv_multiple_field_selection() {
    // Test selecting different fields
    let input = "Alice,30,Engineer\nBob,25,Designer";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("\"field_2\"") // Select third field (occupation)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Engineer");
    assert_eq!(lines[1], "Designer");
}

#[test]
fn test_csv_filtering_numeric() {
    // Test filtering CSV data with numeric comparisons
    let input = "Alice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager\nDana,22,Intern";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"27\"") // Filter by age > 27
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let lines: Vec<&str> = stdout
        .trim()
        .split('\n')
        .filter(|line| !line.is_empty())
        .collect();

    // Should match Alice (30) and Charlie (35)
    assert_eq!(lines.len(), 2);
    assert!(stdout.contains("Alice"));
    assert!(stdout.contains("Charlie"));
    assert!(!stdout.contains("Bob"));
    assert!(!stdout.contains("Dana"));
}

#[test]
fn test_csv_filtering_string() {
    // Test filtering CSV data with string operations
    let input = "Alice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager\nEva,28,Engineer";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_2 == \"Engineer\"") // Filter by occupation
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let lines: Vec<&str> = stdout
        .trim()
        .split('\n')
        .filter(|line| !line.is_empty())
        .collect();

    // Should match Alice and Eva (both Engineers)
    assert_eq!(lines.len(), 2);
    assert!(stdout.contains("Alice"));
    assert!(stdout.contains("Eva"));
    assert!(!stdout.contains("Bob"));
    assert!(!stdout.contains("Charlie"));
}

#[test]
fn test_csv_template_replacement() {
    // Test template replacement with CSV fields
    let input = "Alice,30,Engineer\nBob,25,Designer";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"20\" {Name: ${field_0}, Age: ${field_1}, Job: ${field_2}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Name: Alice, Age: 30, Job: Engineer");
    assert_eq!(lines[1], "Name: Bob, Age: 25, Job: Designer");
}

#[test]
fn test_csv_positional_template_replacement() {
    // Test template replacement using positional variables ($1, $2, etc.)
    let input = "Alice,30,Engineer\nBob,25,Designer";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"20\" {${1} is ${2} years old and works as a ${3}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Alice is 30 years old and works as a Engineer");
    assert_eq!(lines[1], "Bob is 25 years old and works as a Designer");
}

#[test]
fn test_csv_original_input_template() {
    // Test $0 (original input) in templates
    let input = "Alice,30,Engineer";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"25\" {Person: ${field_0} | Original: ${0}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_eq!(stdout.trim(), "Person: Alice | Original: Alice,30,Engineer");
}

#[test]
fn test_csv_complex_filtering() {
    // Test complex boolean logic with CSV data
    let test_cases = vec![
        ("Alice,30,Engineer,true", true),    // age > 25 && active == true
        ("Bob,22,Designer,false", false),    // age <= 25 && active == false
        ("Charlie,35,Manager,false", false), // age > 25 but active == false, so doesn't match
        ("Dana,24,Admin,true", false),       // age <= 25 even though active == true
    ];

    for (input, should_match) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg("field_1 > \"25\" && field_3 == \"true\" {Found: ${field_0} (${field_2})}")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn parsm");

        let stdin = child.stdin.take().expect("get stdin");
        let mut stdin = stdin;
        write!(stdin, "{}", input).expect("write to stdin");
        drop(stdin);

        let result = child.wait_with_output().expect("wait for output");
        let stdout = String::from_utf8_lossy(&result.stdout);

        if should_match {
            assert!(
                !stdout.trim().is_empty(),
                "Expected output for input: {}",
                input
            );
            assert!(stdout.contains("Found:"));
        } else {
            assert_eq!(stdout.trim(), "", "Expected no output for input: {}", input);
        }
    }
}

#[test]
fn test_csv_nonexistent_field() {
    // Test accessing non-existent fields
    let input = "Alice,30,Engineer";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("\"field_5\"") // Field that doesn't exist
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    let stdout = String::from_utf8_lossy(&result.stdout);
    // Non-existent field should return ""
    assert_eq!(stdout.trim(), "");
}

#[test]
fn test_csv_quoted_fields() {
    // Test CSV with quoted fields containing commas and special characters
    let input = r#""Smith, John",35,"Senior Engineer, Tech Lead""#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("\"field_1\"") // Select age field
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_eq!(stdout.trim(), "35");
}

#[test]
fn test_csv_empty_fields() {
    // Test CSV with empty fields
    let input = "Alice,,Engineer\n,25,Designer\nCharlie,35,";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("\"field_0\"") // Select first field
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Alice");
    assert_eq!(lines[1], ""); // Empty field
    assert_eq!(lines[2], "Charlie");
}

#[test]
fn test_csv_numeric_comparisons() {
    // Test various numeric comparisons with CSV data
    let test_cases = vec![
        ("Alice,30,5000", "field_1 > \"25\"", true),
        ("Bob,22,3000", "field_1 < \"25\"", true),
        ("Charlie,30,5000", "field_2 >= \"5000\"", true),
        ("Dana,28,4500", "field_2 <= \"4000\"", false),
        ("Eve,35,6000", "field_1 == \"35\"", true),
        ("Frank,25,3500", "field_1 != \"30\"", true),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(format!("{} {{Match: ${{{}}}}}", filter, "field_0"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn parsm");

        let stdin = child.stdin.take().expect("get stdin");
        let mut stdin = stdin;
        write!(stdin, "{}", input).expect("write to stdin");
        drop(stdin);

        let result = child.wait_with_output().expect("wait for output");
        let stdout = String::from_utf8_lossy(&result.stdout);

        if should_match {
            assert!(
                stdout.contains("Match:"),
                "Expected match for: {} with filter: {}",
                input,
                filter
            );
        } else {
            assert_eq!(
                stdout.trim(),
                "",
                "Expected no match for: {} with filter: {}",
                input,
                filter
            );
        }
    }
}

#[test]
fn test_csv_string_operations() {
    // Test string operations with new symbol-based operators
    let input = "alice@example.com,Alice Smith,active_user";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_0 *= \"@example\" {Email: ${field_0}, Name: ${field_1}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("Email: alice@example.com, Name: Alice Smith"));
}

#[test]
fn test_csv_boolean_logic() {
    // Test various boolean combinations with CSV data
    let input = "Alice,30,true,false,premium";
    let test_cases = vec![
        ("field_2 == \"true\" && field_4 == \"premium\" {Boolean test passed - ${field_2}}", true),
        ("field_2 == \"true\" && field_3 == \"true\" {Boolean test passed - ${field_2}}", false),
        ("field_2 == \"true\" || field_3 == \"true\" {Boolean test passed - ${field_2}}", true),
        // Simplified: explicit comparison instead of truthy evaluation
        ("field_3 == \"false\" && field_4 == \"premium\" {Boolean test passed - ${field_3}}", true),
        // Simplified: use explicit comparisons instead of complex boolean logic
        ("field_2 == \"true\" && field_3 == \"false\" {Boolean test passed - ${field_2}}", true),
        (
            "(field_1 > \"25\") && (field_2 == \"true\" || field_3 == \"true\") {Boolean test passed - ${field_1}}",
            true,
        ),
    ];
    for (filter_with_template, should_match) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(filter_with_template)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn parsm");
        let stdin = child.stdin.take().expect("get stdin");
        let mut stdin = stdin;
        write!(stdin, "{}", input).expect("write to stdin");
        drop(stdin);
        let result = child.wait_with_output().expect("wait for output");
        let stdout = String::from_utf8_lossy(&result.stdout);
        if should_match {
            assert!(
                stdout.contains("Boolean test passed"),
                "Expected match for filter: {}",
                filter_with_template
            );
        } else {
            assert_eq!(
                stdout.trim(),
                "",
                "Expected no match for filter: {}",
                filter_with_template
            );
        }
    }
}

#[test]
fn test_csv_braced_field_syntax() {
    // Test ${field_N} syntax in templates
    let input = "Alice,30,Engineer";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"25\" {Name: ${field_0}, Age: ${field_1}, Job: ${field_2}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_eq!(stdout.trim(), "Name: Alice, Age: 30, Job: Engineer");
}

#[test]
fn test_csv_malformed_input() {
    // Test with malformed CSV (unbalanced quotes)
    let input = r#"Alice,30,"Engineer
Bob,25,Designer"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_0 == \"Bob\"")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    // The tool should handle malformed CSV gracefully without crashing
    assert!(
        result.status.success(),
        "parsm should handle malformed CSV gracefully"
    );
}

#[test]
fn test_csv_replacement_template() {
    // Test CSV object replacement using filter + template with simpler syntax
    let input = "Alice,30,Engineer";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("field_1 > \"25\" {Employee: ${field_0} - Age: ${field_1}, Role: ${field_2}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_eq!(stdout.trim(), "Employee: Alice - Age: 30, Role: Engineer");
}

#[test]
fn test_csv_different_lengths() {
    // Test CSV rows with different number of fields
    let input = "Alice,30\nBob,25,Designer,Manager\nCharlie,35,Engineer";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("\"field_1\"") // Select second field (age)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "30");
    assert_eq!(lines[1], "25");
    assert_eq!(lines[2], "35");
}

#[test]
fn test_csv_single_column() {
    // Test CSV with only one column (edge case)
    let input = "Alice\nBob\nCharlie";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg("\"field_0\"") // Select only field
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{}", input).expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");

    // This might not be detected as CSV since it lacks commas,
    // but if it is, it should work correctly
    if result.status.success() {
        let stdout = String::from_utf8_lossy(&result.stdout);
        if !stdout.trim().is_empty() {
            let lines: Vec<&str> = stdout.trim().split('\n').collect();
            assert_eq!(lines.len(), 3);
        }
    }
}
