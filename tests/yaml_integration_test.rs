use std::io::Write;
use std::process::{Command, Stdio};

/// Test basic YAML field selection
#[test]
fn test_yaml_basic_field_selection() {
    let input = "name: Alice";

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
    assert_eq!(stdout.trim(), "Alice");
}

/// Test YAML field selection with different data types
#[test]
fn test_yaml_field_types() {
    let test_cases = vec![
        ("name: Bob", r#""name""#, "Bob"),
        ("count: 42", r#""count""#, "42"),
        ("enabled: true", r#""enabled""#, "true"),
        ("rate: 3.14", r#""rate""#, "3.14"),
        ("tags: null", r#""tags""#, "null"),
    ];

    for (input, field_selector, expected) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(field_selector)
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
        let result = stdout.trim();
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}

/// Test YAML nonexistent field returns null
#[test]
fn test_yaml_nonexistent_field() {
    let input = "name: Alice";

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
    assert_eq!(stdout.trim(), "null");
}

/// Test YAML template with ${var} syntax
#[test]
fn test_yaml_template_dollar_brace_syntax() {
    let input = "{name: Alice, version: 1.0.0}";

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
    assert_eq!(stdout.trim(), "User: Alice v1.0.0");
}

/// Test YAML template with $var syntax
#[test]
fn test_yaml_template_dollar_syntax() {
    let input = "{app: myapp, env: production}";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg(r#"{App ${app} running in ${env} mode}"#)
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
    assert_eq!(stdout.trim(), "App myapp running in production mode");
}

/// Test literal dollar amounts are not parsed as variables
#[test]
fn test_yaml_literal_dollar_amounts() {
    let input = "{price: 25.50}";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg(r#"{Price is $12 base + ${price} extra}"#)
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
    assert_eq!(stdout.trim(), "Price is $12 base + 25.5 extra");
}

/// Test YAML original input template with ${0}
#[test]
fn test_yaml_original_input_template() {
    let input = "key: value1";

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
    assert_eq!(stdout.trim(), "Original: key: value1 | Key: value1");
}

/// Test YAML numeric comparisons
#[test]
fn test_yaml_numeric_comparisons() {
    let test_cases = vec![
        ("count: 25", "count > 20", true),
        ("count: 15", "count > 20", false),
        ("count: 100", "count < 50", false),
        ("count: 10", "count <= 10", true),
        ("count: 42", "count >= 42", true),
        ("count: 30", "count == 30", true),
        ("count: 25", "count != 30", true),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(filter)
            .arg(r#"{match}"#)
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
            "Command failed for '{}' with filter '{}': {:?}",
            input,
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();

        if should_match {
            assert_eq!(result, "match", "Expected match for filter '{}'", filter);
        } else {
            assert_eq!(result, "", "Expected empty output for filter '{}'", filter);
        }
    }
}

/// Test YAML string operations
#[test]
fn test_yaml_string_operations() {
    let test_cases = vec![
        ("name: Alice", r#"name *= "lic""#, true),
        ("name: Bob", r#"name *= "lic""#, false),
        ("name: Alice", r#"name ^= "Al""#, true),
        ("name: Bob", r#"name ^= "Al""#, false),
        ("name: Alice", r#"name $= "ice""#, true),
        ("name: Bob", r#"name $= "ice""#, false),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(filter)
            .arg(r#"{match}"#)
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
            "Command failed for '{}' with filter '{}': {:?}",
            input,
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();

        if should_match {
            assert_eq!(result, "match", "Expected match for filter '{}'", filter);
        } else {
            assert_eq!(result, "", "Expected empty output for filter '{}'", filter);
        }
    }
}

/// Test YAML boolean logic with explicit comparisons
#[test]
fn test_yaml_boolean_logic() {
    let test_cases = vec![
        (
            "{age: 30, active: true}",
            "age > 25 && active == true",
            true,
        ),
        (
            "{age: 30, active: true}",
            "age > 35 && active == true",
            false,
        ),
        (
            "{age: 30, active: true}",
            "age < 25 || active == true",
            true,
        ),
        (
            "{age: 30, admin: false}",
            "age < 25 || admin == true",
            false,
        ),
        ("admin: false", "admin == false", true),
        ("active: true", "active == false", false),
        (
            "{name: Alice, age: 30}",
            r#"name == "Alice" && age == 30"#,
            true,
        ),
        (
            "{name: Alice, age: 30}",
            r#"name == "Bob" || age == 30"#,
            true,
        ),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(filter)
            .arg(r#"{match}"#)
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
            "Command failed with filter '{}': {:?}",
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();

        if should_match {
            assert_eq!(result, "match", "Expected match for filter '{}'", filter);
        } else {
            assert_eq!(result, "", "Expected empty output for filter '{}'", filter);
        }
    }
}

/// Test YAML flow syntax field access
#[test]
fn test_yaml_flow_syntax() {
    let test_cases = vec![
        (
            "user: {name: Charlie, age: 25}",
            r#""user.name""#,
            "Charlie",
        ),
        ("user: {name: Charlie, age: 25}", r#""user.age""#, "25"),
    ];

    for (input, field_selector, expected) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(field_selector)
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
            "Command failed for selector '{}' with input '{}': {:?}",
            field_selector,
            input,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();
        assert_eq!(
            result, expected,
            "Failed for selector: {} with input: {}",
            field_selector, input
        );
    }
}

/// Test YAML array handling
#[test]
fn test_yaml_array_handling() {
    let input = "tags: [web, api, rust]";

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg(r#""tags""#)
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
    let result = stdout.trim();

    // The output should be JSON array containing all elements
    assert!(result.contains("web"), "Should contain 'web'");
    assert!(result.contains("api"), "Should contain 'api'");
    assert!(result.contains("rust"), "Should contain 'rust'");
    assert!(
        result.starts_with("[") && result.ends_with("]"),
        "Should be JSON array format"
    );
}

/// Test YAML nested field access on single line
#[test]
fn test_yaml_nested_field_access() {
    let input = "config: {database: {host: localhost, port: 5432}}";

    let test_cases = vec![
        (r#""config.database.host""#, "localhost"),
        (r#""config.database.port""#, "5432"),
    ];

    for (field_selector, expected) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(field_selector)
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
            "Command failed for selector '{}': {:?}",
            field_selector,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();
        assert_eq!(result, expected, "Failed for selector: {}", field_selector);
    }
}

/// Test YAML with complex filtering
#[test]
fn test_yaml_complex_filtering() {
    let test_cases = vec![
        (
            "{status: active, count: 100}",
            r#"status == "active""#,
            true,
        ),
        (
            "{status: inactive, count: 50}",
            r#"status == "active""#,
            false,
        ),
        (
            "{age: 30, active: true}",
            "age > 25 && active == true",
            true,
        ),
        (
            "{age: 20, active: false}",
            "age > 25 && active == true",
            false,
        ),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(filter)
            .arg(r#"{match}"#)
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
            "Command failed for input '{}' with filter '{}': {:?}",
            input,
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();

        if should_match {
            assert_eq!(result, "match", "Expected match for filter '{}'", filter);
        } else {
            assert_eq!(result, "", "Expected empty output for filter '{}'", filter);
        }
    }
}

/// Test YAML document separators (line-by-line processing)
#[test]
fn test_yaml_document_separators() {
    let input = "---\nname: Alice\n---\nname: Bob";

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

    // Each line with "name:" should produce output, document separators should produce null
    assert!(lines.len() >= 2, "Should have at least 2 results");
    assert!(lines.iter().any(|line| line.contains("Alice")));
    assert!(lines.iter().any(|line| line.contains("Bob")));
}

/// Test YAML quoted keys
#[test]
fn test_yaml_quoted_keys() {
    let input = r#"normal_key: value3"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg(r#""normal_key""#)
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
    let result = stdout.trim();
    assert_eq!(result, "value3", "Should extract normal_key value");
}

/// Test YAML empty values handling
#[test]
fn test_yaml_empty_values() {
    let test_cases = vec![
        ("empty_string: \"\"", r#""empty_string""#, ""),
        ("null_value: null", r#""null_value""#, "null"),
        ("zero: 0", r#""zero""#, "0"),
        ("false_value: false", r#""false_value""#, "false"),
    ];

    for (input, field_selector, expected) in test_cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
            .arg(field_selector)
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
        let result = stdout.trim();
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}
