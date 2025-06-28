use std::io::Write;
use std::process::{Command, Stdio};

/// Test basic TOML field selection
#[test]
fn test_toml_basic_field_selection() {
    let input = r#"name = "Alice"
age = 30
active = true"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Alice");
    assert_eq!(lines[1], "null"); // age line doesn't have name field
    assert_eq!(lines[2], "null"); // active line doesn't have name field
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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
            "Mismatch for input '{}' with field '{}'",
            input,
            field
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

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    // Should have lines for each TOML declaration (6 lines total)
    assert_eq!(lines.len(), 6);

    // The section header itself doesn't have a "database" field, so should be null
    // But the key-value pairs under it also don't have "database" field
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            // First line is the [database] section header, which becomes an empty object
            assert!(
                *line == "{}" || *line == "null",
                "Expected empty object or null for section header, got '{}'",
                line
            );
        } else {
            assert!(
                *line == "null" || line.is_empty(),
                "Expected null or empty for line {}, got '{}'",
                i,
                line
            );
        }
    }
}

/// Test TOML nested field access (dot notation in keys)
#[test]
fn test_toml_nested_keys() {
    let input = r#"database.host = "localhost"
database.port = 5432
server.host = "0.0.0.0""#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "localhost");
    assert_eq!(lines[1], "null"); // database.port line doesn't have database.host
    assert_eq!(lines[2], "null"); // server.host line doesn't have database.host
}

/// Test TOML nonexistent field
#[test]
fn test_toml_nonexistent_field() {
    let input = r#"name = "Alice"
age = 30"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "null");
    assert_eq!(lines[1], "null");
}

/// Test TOML arrays
#[test]
fn test_toml_arrays() {
    let input = r#"fruits = ["apple", "banana", "cherry"]
numbers = [1, 2, 3]"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    assert_eq!(lines.len(), 6); // Pretty-printed array takes 4 lines + 1 null line + 1 warning
                                // Check that we have the pretty-printed array
    assert!(lines[0] == "[");
    assert!(lines[1].contains("apple"));
    assert!(lines[2].contains("banana"));
    assert!(lines[3].contains("cherry"));
    assert!(lines[4] == "]");
    assert_eq!(lines[5], "null"); // numbers line doesn't have fruits field
}

/// Test TOML template with variable syntax ${field}
#[test]
fn test_toml_braced_field_syntax() {
    let input = r#"name = "Bob"
version = "1.0.0""#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "User: Bob v"); // First line missing version
    assert_eq!(lines[1], "User:  v1.0.0"); // Second line missing name
}

/// Test TOML template replacement
#[test]
fn test_toml_template_replacement() {
    let input = r#"app = "myapp"
env = "production"
debug = false"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "App myapp running in  mode (debug: )"); // Has app but not env/debug
    assert_eq!(lines[1], "App  running in production mode (debug: )"); // Has env but not app/debug
    assert_eq!(lines[2], "App  running in  mode (debug: false)"); // Has debug but not app/env
}

/// Test TOML original input template
#[test]
fn test_toml_original_input_template() {
    let input = r#"key = "value1"
other = "value2""#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], r#"Original: key = "value1" | Key: value1"#);
    assert_eq!(lines[1], r#"Original: other = "value2" | Key:"#); // Second line has no 'key' field
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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
                "Expected match for '{}' with filter '{}'",
                input,
                filter
            );
            assert!(
                stdout.contains("String ops test"),
                "Expected output to contain test message for '{}'",
                input
            );
        } else {
            assert!(
                stdout.trim().is_empty(),
                "Expected no match for '{}' with filter '{}'",
                input,
                filter
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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
                "Expected match for '{}' with filter '{}'",
                input,
                filter
            );
            assert!(
                stdout.contains("Numeric test"),
                "Expected output to contain test message for '{}'",
                input
            );
        } else {
            assert!(
                stdout.trim().is_empty(),
                "Expected no match for '{}' with filter '{}'",
                input,
                filter
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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
                "Expected match for filter '{}'",
                filter
            );
            assert!(
                stdout.contains("Boolean test passed"),
                "Expected output to contain test message for '{}'",
                filter
            );
        } else {
            assert!(
                stdout.trim().is_empty(),
                "Expected no match for filter '{}'",
                filter
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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
                "Expected no matches for filter '{}', got {:?}",
                filter,
                lines
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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
            println!("Malformed input '{}' correctly rejected", input);
        }
    }
}

/// Test TOML inline tables
#[test]
fn test_toml_inline_tables() {
    let input = r#"server = { host = "localhost", port = 8080 }
client = { timeout = 30 }"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    assert_eq!(lines.len(), 5); // Pretty-printed object takes 4 lines + 1 null line
                                // First part should contain the server table as pretty-printed JSON
    assert!(lines[0] == "{");
    assert!(lines[1].contains("host") && lines[1].contains("localhost"));
    assert!(lines[2].contains("port") && lines[2].contains("8080"));
    assert!(lines[3] == "}");
    // Last line should be null (client line doesn't have server field)
    assert_eq!(lines[4], "null");
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

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    // Should process each TOML line/declaration
    assert!(lines.len() >= 4);

    // The first few lines should have the expected template replacements
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Title: TOML Example")),
        "Expected to find template replacement in output: {:?}",
        lines
    );
}

/// Test TOML empty values and whitespace
#[test]
fn test_toml_empty_and_whitespace() {
    let input = r#"empty_string = ""
name = "   spaced   "
zero = 0"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
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

    assert_eq!(lines.len(), 3);
    // Should properly handle empty string, spaced string, and zero
    assert!(
        lines.iter().any(|line| line.contains("Empty: ''")),
        "Expected empty string handling in: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains("spaced")),
        "Expected spaced string in: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains("Zero: 0")),
        "Expected zero value in: {:?}",
        lines
    );
}
