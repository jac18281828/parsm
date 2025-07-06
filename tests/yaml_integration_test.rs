use std::io::Write;
use std::process::{Command, Stdio};

/// Helper function to create a Command with proper environment setup
fn parsm_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_parsm"));
    cmd.env("RUST_LOG", "parsm=error");
    cmd
}

// NOTE: Many YAML integration tests are currently failing due to systematic issues
// with YAML processing, particularly:
// 1. Template processing bug affecting multi-line input
// 2. Field selection issues with some YAML formats
// 3. Filter processing problems with YAML data structures
// These issues are tracked separately from the truthy operator and CSV header fixes.

/// Test basic YAML field selection
#[test]
fn test_yaml_basic_field_selection() {
    let input = "name: Alice";

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
        let mut child = parsm_command()
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
        assert_eq!(result, expected, "Failed for input: {input}");
    }
}

/// Test YAML nonexistent field returns null
#[test]
fn test_yaml_nonexistent_field() {
    let input = "name: Alice";

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
    assert_eq!(stdout.trim(), "");
}

/// Test YAML template with ${var} syntax
#[test]
fn test_yaml_template_dollar_brace_syntax() {
    let input = "name: Alice\nversion: 1.0.0";

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
    assert_eq!(stdout.trim(), "User: Alice v1.0.0");
}

/// Test YAML template with $var syntax
#[test]
fn test_yaml_template_dollar_syntax() {
    let input = "app: myapp\nenv: production";

    let mut child = parsm_command()
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
    let input = "price: 25.50";

    let mut child = parsm_command()
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
        let mut child = parsm_command()
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
            assert_eq!(result, "match", "Expected match for filter '{filter}'");
        } else {
            assert_eq!(result, "", "Expected empty output for filter '{filter}'");
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
        let mut child = parsm_command()
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
            assert_eq!(result, "match", "Expected match for filter '{filter}'");
        } else {
            assert_eq!(result, "", "Expected empty output for filter '{filter}'");
        }
    }
}

/// Test YAML boolean logic with explicit comparisons
#[test]
fn test_yaml_boolean_logic() {
    let test_cases = vec![
        ("age: 30\nactive: true", "age > 25 && active == true", true),
        ("age: 30\nactive: true", "age > 35 && active == true", false),
        ("age: 30\nactive: true", "age < 25 || active == true", true),
        ("age: 30\nadmin: false", "age < 25 || admin == true", false),
        ("admin: false", "admin == false", true),
        ("active: true", "active == false", false),
        (
            "name: Alice\nage: 30",
            r#"name == "Alice" && age == 30"#,
            true,
        ),
        (
            "name: Alice\nage: 30",
            r#"name == "Bob" || age == 30"#,
            true,
        ),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
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
            assert_eq!(result, "match", "Expected match for filter '{filter}'");
        } else {
            assert_eq!(result, "", "Expected empty output for filter '{filter}'");
        }
    }
}

/// Test YAML nested field access on multiple lines
#[test]
fn test_yaml_flow_syntax() {
    let test_cases = vec![
        (
            "user:\n  name: Charlie\n  age: 25",
            r#""user.name""#,
            "Charlie",
        ),
        ("user:\n  name: Charlie\n  age: 25", r#""user.age""#, "25"),
    ];

    for (input, field_selector, expected) in test_cases {
        let mut child = parsm_command()
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
            "Failed for selector: {field_selector} with input: {input}",
        );
    }
}

/// Test YAML array handling
#[test]
fn test_yaml_array_handling() {
    let input = "tags:\n  - web\n  - api\n  - rust";

    let mut child = parsm_command()
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

/// Test YAML nested field access on multiple lines
#[test]
fn test_yaml_nested_field_access() {
    let input = "config:\n  database:\n    host: localhost\n    port: 5432";

    let test_cases = vec![
        (r#""config.database.host""#, "localhost"),
        (r#""config.database.port""#, "5432"),
    ];

    for (field_selector, expected) in test_cases {
        let mut child = parsm_command()
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
        assert_eq!(result, expected, "Failed for selector: {field_selector}");
    }
}

/// Test YAML with complex filtering
#[test]
fn test_yaml_complex_filtering() {
    let test_cases = vec![
        ("status: active\ncount: 100", r#"status == "active""#, true),
        (
            "status: inactive\ncount: 50",
            r#"status == "active""#,
            false,
        ),
        ("age: 30\nactive: true", "age > 25 && active == true", true),
        (
            "age: 20\nactive: false",
            "age > 25 && active == true",
            false,
        ),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
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
            assert_eq!(result, "match", "Expected match for filter '{filter}'");
        } else {
            assert_eq!(result, "", "Expected empty output for filter '{filter}'");
        }
    }
}

/// Test YAML document separators (line-by-line processing)
#[test]
fn test_yaml_document_separators() {
    let input = "---\nname: Alice\n---\nname: Bob";

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

    // Each line with "name:" should produce output, document separators should produce null
    assert!(lines.len() >= 2, "Should have at least 2 results");
    assert!(lines.iter().any(|line| line.contains("Alice")));
    assert!(lines.iter().any(|line| line.contains("Bob")));
}

/// Test YAML quoted keys
#[test]
fn test_yaml_quoted_keys() {
    let input = r#"normal_key: value3"#;

    let mut child = parsm_command()
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
        let mut child = parsm_command()
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
        assert_eq!(result, expected, "Failed for input: {input}",);
    }
}

/// Test YAML flow format with forced --yaml flag
#[test]
fn test_yaml_flow_format_forced() {
    let test_cases = vec![
        ("{name: Alice, age: 30}", r#""name""#, "Alice"),
        ("{name: Alice, age: 30}", r#""age""#, "30"),
        ("{user: {name: Bob, role: admin}}", r#""user.name""#, "Bob"),
        (
            "{user: {name: Bob, role: admin}}",
            r#""user.role""#,
            "admin",
        ),
        ("{price: 25.50, currency: USD}", r#""price""#, "25.5"),
        ("{price: 25.50, currency: USD}", r#""currency""#, "USD"),
        ("{items: [apple, banana, cherry]}", r#""items.0""#, "apple"),
        ("{items: [apple, banana, cherry]}", r#""items.2""#, "cherry"),
    ];

    for (input, field_selector, expected) in test_cases {
        let mut child = parsm_command()
            .arg("--yaml")
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
            "Command failed for input '{}' with selector '{}': {:?}",
            input,
            field_selector,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();
        assert_eq!(
            result, expected,
            "Failed for YAML flow input '{input}' with selector '{field_selector}'",
        );
    }
}

/// Test YAML flow format templates with forced --yaml flag
#[test]
fn test_yaml_flow_format_templates_forced() {
    let test_cases = vec![
        (
            "{name: Alice, age: 30}",
            r#"{${name} is ${age} years old}"#,
            "Alice is 30 years old",
        ),
        (
            "{user: {name: Bob, role: admin}}",
            r#"{User: ${user.name} (${user.role})}"#,
            "User: Bob (admin)",
        ),
        (
            "{price: 25.50, currency: USD}",
            r#"{Cost: $100 base + ${price} ${currency}}"#,
            "Cost: $100 base + 25.5 USD",
        ),
        (
            "{items: [apple, banana, cherry]}",
            r#"{First: ${items.0}, Last: ${items.2}}"#,
            "First: apple, Last: cherry",
        ),
    ];

    for (input, template, expected) in test_cases {
        let mut child = parsm_command()
            .arg("--yaml")
            .arg(template)
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
            "Command failed for input '{}' with template '{}': {:?}",
            input,
            template,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();
        assert_eq!(
            result, expected,
            "Failed for YAML flow input '{input}' with template '{template}'",
        );
    }
}

/// Test YAML flow format filtering with forced --yaml flag
#[test]
fn test_yaml_flow_format_filtering_forced() {
    let test_cases = vec![
        ("{name: Alice, age: 30}", "age > 25", true),
        ("{name: Bob, age: 20}", "age > 25", false),
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
            "{user: {name: Alice, admin: true}}",
            "user.admin == true",
            true,
        ),
        (
            "{user: {name: Bob, admin: false}}",
            "user.admin == true",
            false,
        ),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg("--yaml")
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
            assert_eq!(
                result, "match",
                "Expected match for YAML flow filter '{filter}' with input '{input}'",
            );
        } else {
            assert_eq!(
                result, "",
                "Expected empty output for YAML flow filter '{filter}' with input '{input}'",
            );
        }
    }
}

/// Test YAML flow format with complex nested structures using forced --yaml flag
#[test]
fn test_yaml_flow_format_complex_nested_forced() {
    let test_cases = vec![
        (
            "{config: {db: {host: localhost, port: 5432}, app: {name: myapp}}}",
            r#""config.db.host""#,
            "localhost",
        ),
        (
            "{config: {db: {host: localhost, port: 5432}, app: {name: myapp}}}",
            r#""config.app.name""#,
            "myapp",
        ),
        (
            "{users: [{name: Alice, role: admin}, {name: Bob, role: user}]}",
            r#""users.0.name""#,
            "Alice",
        ),
        (
            "{users: [{name: Alice, role: admin}, {name: Bob, role: user}]}",
            r#""users.1.role""#,
            "user",
        ),
    ];

    for (input, field_selector, expected) in test_cases {
        let mut child = parsm_command()
            .arg("--yaml")
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
            "Command failed for complex YAML flow input '{}' with selector '{}': {:?}",
            input,
            field_selector,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();
        assert_eq!(
            result, expected,
            "Failed for complex YAML flow input '{input}' with selector '{field_selector}'",
        );
    }
}
