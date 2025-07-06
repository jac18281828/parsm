use std::io::Write;
use std::process::{Command, Stdio};

/// Helper function to create a Command with proper environment setup
fn parsm_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_parsm"));
    cmd.env("RUST_LOG", "parsm=error");
    cmd
}

/// Test basic TOML field selection
#[test]
fn test_toml_basic_field_selection() {
    let input = r#"name = "Alice"
age = 30
active = true"#;

    let mut child = parsm_command()
        .arg(r#""name""#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With proper TOML parsing, we get a single result for the field extraction
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "Alice");
}

/// Test TOML field selection with different data types
#[test]
fn test_toml_field_types() {
    let test_cases = vec![
        (r#"name = "Bob""#, r#""name""#, "Bob"),
        (r#"count = 42"#, r#""count""#, "42"),
        (r#"enabled = true"#, r#""enabled""#, "true"),
        (r#"rate = 3.14"#, r#""rate""#, "3.14"),
    ];

    for (input, field, expected) in test_cases {
        let mut child = parsm_command()
            .arg(field)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start parsm");

        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(input.as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read stdout");
        assert!(
            output.status.success(),
            "Command failed for input '{}': {:?}",
            input,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.trim(),
            expected,
            "Mismatch for input '{input}' with field '{field}'",
        );
    }
}

/// Test TOML section handling
#[test]
fn test_toml_sections() {
    let input = r#"[database]
host = "localhost"
port = 5432

[server]
host = "0.0.0.0"
port = 8080"#;

    let mut child = parsm_command()
        .arg(r#""database""#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With document-level TOML parsing, we get the database section as pretty-printed JSON
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "{");
    assert!(lines[1].contains("host") && lines[1].contains("localhost"));
    assert!(lines[2].contains("port") && lines[2].contains("5432"));
    assert_eq!(lines[3], "}");
}

/// Test TOML nested field access (dot notation in keys)
#[test]
fn test_toml_nested_keys() {
    let input = r#"database.host = "localhost"
database.port = 5432
server.host = "0.0.0.0""#;

    let mut child = parsm_command()
        .arg(r#""database.host""#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With document-level TOML parsing, we get one result for the nested key
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "localhost");
}

/// Test TOML nonexistent field
#[test]
fn test_toml_nonexistent_field() {
    let input = r#"name = "Alice"
age = 30"#;

    let mut child = parsm_command()
        .arg(r#""nonexistent""#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With document-level TOML parsing, nonexistent field returns empty output
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "");
}

/// Test TOML arrays
#[test]
fn test_toml_arrays() {
    let input = r#"fruits = ["apple", "banana", "cherry"]
numbers = [1, 2, 3]"#;

    let mut child = parsm_command()
        .arg(r#""fruits""#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With document-level TOML parsing, we get the fruits array formatted as JSON
    assert!(lines.len() >= 4); // Pretty-printed array takes multiple lines
                               // Check that we have the pretty-printed array
    assert!(lines[0] == "[");
    assert!(lines[1].contains("apple"));
    assert!(lines[2].contains("banana"));
    assert!(lines[3].contains("cherry"));
    assert!(lines[4] == "]");
}

/// Test TOML template with variable syntax ${field}
#[test]
fn test_toml_braced_field_syntax() {
    let input = r#"name = "Bob"
version = "1.0.0""#;

    let mut child = parsm_command()
        .arg(r#"{User: ${name} v${version}}"#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With document-level TOML parsing, we get a single template result
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "User: Bob v1.0.0");
}

/// Test TOML template replacement
#[test]
fn test_toml_template_replacement() {
    let input = r#"app = "myapp"
env = "production"
debug = false"#;

    let mut child = parsm_command()
        .arg(r#"{App ${app} running in ${env} mode (debug: ${debug})}"#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With document-level TOML parsing, we get a single template result
    assert_eq!(lines.len(), 1);
    assert_eq!(
        lines[0],
        "App myapp running in production mode (debug: false)"
    );
}

/// Test TOML original input template
#[test]
fn test_toml_original_input_template() {
    let input = r#"key = "value1"
other = "value2""#;

    let mut child = parsm_command()
        .arg(r#"{Original: ${0} | Key: ${key}}"#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With proper TOML document parsing, the ${0} contains newlines so we get 2 output lines
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], r#"Original: key = "value1""#);
    assert_eq!(lines[1], r#"other = "value2" | Key: value1"#);
}

/// Test TOML string operations with filtering
#[test]
fn test_toml_string_operations() {
    let test_cases = vec![
        (r#"name = "alice""#, r#"name == "alice""#, true),
        (r#"name = "BOB""#, r#"name == "BOB""#, true),
        (
            r#"title = "Hello World""#,
            r#"title == "Hello World""#,
            true,
        ),
        (r#"msg = "test""#, r#"msg *= "es""#, true),
        (r#"msg = "hello""#, r#"msg ^= "hel""#, true),
        (r#"msg = "world""#, r#"msg $= "rld""#, true),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg(filter)
            .arg(r#"{String ops test}"#)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start parsm");

        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(input.as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read stdout");
        assert!(
            output.status.success(),
            "Command failed for input '{}': {:?}",
            input,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        if should_match {
            assert!(
                !stdout.trim().is_empty(),
                "Expected match for '{input}' with filter '{filter}'"
            );
            assert!(
                stdout.contains("String ops test"),
                "Expected output to contain test message for '{input}'"
            );
        } else {
            assert!(
                stdout.trim().is_empty(),
                "Expected no match for '{input}' with filter '{filter}'"
            );
        }
    }
}

/// Test TOML numeric comparisons
#[test]
fn test_toml_numeric_comparisons() {
    let test_cases = vec![
        (r#"count = 42"#, r#"count > 40"#, true),
        (r#"count = 25"#, r#"count < 30"#, true),
        (r#"count = 50"#, r#"count == 50"#, true),
        (r#"count = 15"#, r#"count >= 15"#, true),
        (r#"count = 10"#, r#"count <= 20"#, true),
        (r#"count = 5"#, r#"count != 10"#, true),
        (r#"count = 100"#, r#"count < 50"#, false),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg(filter)
            .arg(r#"{Numeric test}"#)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start parsm");

        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(input.as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read stdout");
        assert!(
            output.status.success(),
            "Command failed for input '{}': {:?}",
            input,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        if should_match {
            assert!(
                !stdout.trim().is_empty(),
                "Expected match for '{input}' with filter '{filter}'"
            );
            assert!(
                stdout.contains("Numeric test"),
                "Expected output to contain test message for '{input}'"
            );
        } else {
            assert!(
                stdout.trim().is_empty(),
                "Expected no match for '{input}' with filter '{filter}'"
            );
        }
    }
}

/// Test TOML boolean logic
#[test]
fn test_toml_boolean_logic() {
    let test_cases = vec![
        (r#"active = true"#, r#"active == true"#, true),
        (r#"active = false"#, r#"active == false"#, true),
        (r#"active = true"#, r#"active == false"#, false),
        (r#"enabled = false"#, r#"enabled == false"#, true),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg(filter)
            .arg(r#"{Boolean test passed}"#)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start parsm");

        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(input.as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read stdout");
        assert!(
            output.status.success(),
            "Command failed for filter '{}': {:?}",
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        if should_match {
            assert!(
                !stdout.trim().is_empty(),
                "Expected match for filter '{filter}'"
            );
            assert!(
                stdout.contains("Boolean test passed"),
                "Expected output to contain test message for '{filter}'"
            );
        } else {
            assert!(
                stdout.trim().is_empty(),
                "Expected no match for filter '{filter}'"
            );
        }
    }
}

/// Test TOML complex filtering with multiple conditions
#[test]
fn test_toml_complex_filtering() {
    let input = r#"name = "alice"
age = 25
active = true
score = 85.5"#;

    let test_cases = vec![
        (r#"age > 20"#, true),        // This will match the age line
        (r#"name == "alice""#, true), // This will match the name line
        (r#"score > 80"#, true),      // This will match the score line
        (r#"age > 30"#, false),       // This won't match (age is 25)
    ];

    for (filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg(filter)
            .arg(r#"{Complex filter result}"#)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start parsm");

        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(input.as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read stdout");
        assert!(
            output.status.success(),
            "Command failed for filter '{}': {:?}",
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.trim().split('\n').collect();

        if should_match {
            // Should match 1 line that has the relevant field
            assert_eq!(
                lines.len(),
                1,
                "Expected 1 match for filter '{}', got {}",
                filter,
                lines.len()
            );
            assert!(
                lines[0].contains("Complex filter result"),
                "Expected output to contain test message in line '{}'",
                lines[0]
            );
        } else {
            // Should match no lines (empty output)
            assert!(
                lines.is_empty() || (lines.len() == 1 && lines[0].is_empty()),
                "Expected no matches for filter '{filter}', got {lines:?}",
            );
        }
    }
}

/// Test TOML malformed input handling
#[test]
fn test_toml_malformed_input() {
    let malformed_inputs = vec![
        "name =",     // incomplete assignment
        "= value",    // missing key
        "name value", // missing equals
        "[section",   // incomplete section
    ];

    for input in malformed_inputs {
        let mut child = parsm_command()
            .arg(r#""name""#)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start parsm");

        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(input.as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to read stdout");

        // For malformed TOML, the tool should either:
        // 1. Exit with success but produce no/null output (fallback to text mode)
        // 2. Exit with error on the first line

        if output.status.success() {
            // If successful, output should be empty or null
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Should either be empty or contain fallback text processing
            println!(
                "Malformed input '{}' processed as: '{}'",
                input,
                stdout.trim()
            );
        } else {
            // If failed, that's also acceptable for malformed input
            println!("Malformed input '{input}' correctly rejected");
        }
    }
}

/// Test TOML inline tables
#[test]
fn test_toml_inline_tables() {
    let input = r#"server = { host = "localhost", port = 8080 }
client = { timeout = 30 }"#;

    let mut child = parsm_command()
        .arg(r#""server""#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With document-level TOML parsing, we get the server object formatted as pretty-printed JSON
    assert!(lines.len() >= 3); // Pretty-printed object takes multiple lines
                               // First part should contain the server table as pretty-printed JSON
    assert!(lines[0] == "{");
    assert!(lines[1].contains("host") && lines[1].contains("localhost"));
    assert!(lines[2].contains("port") && lines[2].contains("8080"));
    assert!(lines[3] == "}");
}

/// Test TOML with mixed data types and complex structures
#[test]
fn test_toml_complex_structures() {
    let input = r#"title = "TOML Example"
enabled = true
count = 42
items = ["first", "second"]

[database]
connection_timeout = 5000
retry_count = 3"#;

    let mut child = parsm_command()
        .arg(r#"{Title: ${title}, Count: ${count}, Enabled: ${enabled}}"#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With document-level TOML parsing, we get a single template result
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "Title: TOML Example, Count: 42, Enabled: true");
}

/// Test TOML empty values and whitespace
#[test]
fn test_toml_empty_and_whitespace() {
    let input = r#"empty_string = ""
name = "   spaced   "
zero = 0"#;

    let mut child = parsm_command()
        .arg(r#"{Empty: '${empty_string}', Name: '${name}', Zero: ${zero}}"#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start parsm");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // With document-level TOML parsing, we get a single template result
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "Empty: '', Name: '   spaced   ', Zero: 0");
}
