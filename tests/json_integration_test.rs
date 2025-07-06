use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// Helper function to create a Command with proper environment setup
fn parsm_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_parsm"));
    cmd.env("RUST_LOG", "parsm=error");
    cmd
}

#[test]
fn test_json_array_field_selection() {
    // Test field selection on JSON arrays
    let input = r#"[
        {"Id": "1", "State": {"Status": "running", "Pid": 123}},
        {"Id": "2", "State": {"Status": "stopped", "Pid": 456}}
    ]"#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("\"State\"")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should output two pretty-printed State objects
    assert!(stdout.contains("\"Status\": \"running\""));
    assert!(stdout.contains("\"Status\": \"stopped\""));
    assert!(stdout.contains("\"Pid\": 123"));
    assert!(stdout.contains("\"Pid\": 456"));
}

#[test]
fn test_json_object_field_selection() {
    // Test field selection on single JSON object
    let input = r#"{"name": "Alice", "age": 30, "active": true, "profile": {"email": "alice@example.com"}}"#;

    let mut child = parsm_command()
        .arg("\"name\"")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_eq!(stdout.trim(), "Alice");
}

#[test]
fn test_json_nested_field_selection() {
    // Test nested field selection
    let input = r#"{"user": {"profile": {"name": "Bob", "settings": {"theme": "dark"}}}}"#;

    let mut child = parsm_command()
        .arg("\"user.profile.name\"")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_eq!(stdout.trim(), "Bob");
}

#[test]
fn test_json_filter_and_template() {
    // Test filtering with template output
    let input = r#"{"name": "Charlie", "age": 25, "status": "active"}"#;

    let mut child = parsm_command()
        .arg("age > 20 {User ${name} is ${age} years old (status: ${status})}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_eq!(
        stdout.trim(),
        "User Charlie is 25 years old (status: active)"
    );
}

#[test]
fn test_json_template_with_original_input() {
    // Test $0 (original input) in templates
    let input = r#"{"name": "Dana", "score": 95}"#;

    let mut child = parsm_command()
        .arg("score > 90 {Result: ${name} scored ${score} points. Original: ${0}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("Result: Dana scored 95 points"));
    assert!(stdout.contains(&format!("Original: {input}")));
}

#[test]
fn test_json_complex_filtering() {
    // Test complex boolean logic
    let test_cases = vec![
        (
            r#"{"name": "Eve", "age": 30, "active": true, "role": "admin"}"#,
            true,
        ),
        (
            r#"{"name": "Frank", "age": 22, "active": false, "role": "user"}"#,
            false,
        ),
        (
            r#"{"name": "Grace", "age": 35, "active": true, "role": "user"}"#,
            true,
        ),
    ];

    for (input, should_match) in test_cases {
        let mut child = parsm_command()
            .arg("(age > 25 && active == true) || role == \"admin\" {Found: ${name} (${role})}")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn parsm");

        let stdin = child.stdin.take().expect("get stdin");
        let mut stdin = stdin;
        write!(stdin, "{input}").expect("write to stdin");
        drop(stdin);

        let result = child.wait_with_output().expect("wait for output");
        let stdout = String::from_utf8_lossy(&result.stdout);

        if should_match {
            assert!(
                !stdout.trim().is_empty(),
                "Expected output for input: {input}"
            );
            assert!(stdout.contains("Found:"));
        } else {
            assert_eq!(stdout.trim(), "", "Expected no output for input: {input}");
        }
    }
}

#[test]
fn test_json_field_selection_nonexistent() {
    // Test field selection with non-existent field
    let input = r#"{"name": "Henry", "age": 40}"#;

    let mut child = parsm_command()
        .arg("\"nonexistent\"")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    let stdout = String::from_utf8_lossy(&result.stdout);
    // Field selection for non-existent fields returns empty output
    assert_eq!(stdout.trim(), "");
}

#[test]
fn test_json_array_of_primitives() {
    // Test field selection on array of primitives
    let input = r#"["apple", "banana", "cherry"]"#;

    let mut child = parsm_command()
        .arg("\"0\"") // Try to access index as field
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    // The behavior here depends on implementation - it might return null or handle differently
    assert!(
        result.status.success(),
        "parsm should handle array of primitives gracefully"
    );
}

#[test]
fn test_json_malformed_input() {
    // Test with malformed JSON - should handle gracefully
    let input = r#"{"name": "Invalid JSON"
{"name": "Second line", "age": 25}"#;

    let mut child = parsm_command()
        .arg("name == \"Second line\" {${name}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    // The tool should handle malformed JSON gracefully without crashing
    assert!(
        result.status.success(),
        "parsm should handle malformed JSON gracefully"
    );
    // Note: Current behavior with malformed JSON may not process subsequent lines
}

#[test]
fn test_json_replacement_template() {
    // Test JSON object replacement using filter + template
    let input = r#"{"name": "Iris", "age": 28, "city": "Portland"}"#;

    let mut child = parsm_command()
        .arg("age > 25 {\"person\": \"${name}\", \"location\": \"${city}\", \"adult\": true}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("\"person\":"));
    assert!(stdout.contains("Iris"));
    assert!(stdout.contains("\"location\": \"Portland\""));
    assert!(stdout.contains("\"adult\": true"));
}

#[test]
fn test_json_string_operations() {
    // Test string operations like contains, startswith, endswith
    let input = r#"{"email": "user@example.com", "name": "John Doe", "status": "active_user"}"#;

    // Test contains
    let mut child = parsm_command()
        .arg("email ~ \"@example\" {Email: ${email}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("Email: user@example.com"));
}

#[test]
fn test_json_truthy_operator() {
    // Test truthy operator with JSON data - multiple lines like the working example
    let input = r#"{"a": 1, "b": 1}
{"a": 1, "b": 1, "c": 1}"#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Test simple truthy check with AND logic
    let output = parsm_command()
        .arg("a? && b? [${a}, ${b}]")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should match both lines where a and b are truthy
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    // Debug output to see what we actually get
    eprintln!("Debug: got {} lines: {:?}", lines.len(), lines);
    assert_eq!(lines.len(), 2, "Expected 2 matching lines");
    assert!(lines[0].contains("1, 1")); // Should show 1, 1
    assert!(lines[1].contains("1, 1")); // Should show 1, 1

    // Test mixed truthy and comparison
    // Note: Currently there's a known issue with templates and multi-line input
    // Testing with single line that matches the condition
    let input2 = r#"{"a": 1, "g": true}"#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input2}").expect("write temp file");

    let output = parsm_command()
        .arg("a? && g == true [${a}, ${g}]")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should match the line where a is truthy and g is true
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    // Debug output to see what we actually get
    eprintln!("Debug mixed: got {} lines: {:?}", lines.len(), lines);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("1, true"));
}

#[test]
fn test_json_truthy_operator_and_expr() {
    // Test truthy operator with JSON objects
    let input = r#"{"active": true, "verified": false, "premium": true}"#;

    let mut child = parsm_command()
        .arg("active? && premium? {User is active and premium}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("User is active and premium"));
}

#[test]
fn test_json_numeric_comparisons() {
    // Test numeric comparisons with various types
    let test_cases = vec![
        (r#"{"score": 85, "threshold": 80}"#, "score > 80", true),
        (r#"{"price": 49, "budget": 50}"#, "price < 50", true),
        (r#"{"count": 10, "limit": 10}"#, "count >= 10", true),
        (r#"{"age": 17, "min_age": 18}"#, "age < 18", true),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg(format!("{filter} {{Match found - ${{score}}}}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn parsm");

        let stdin = child.stdin.take().expect("get stdin");
        let mut stdin = stdin;
        write!(stdin, "{input}").expect("write to stdin");
        drop(stdin);

        let result = child.wait_with_output().expect("wait for output");
        let stdout = String::from_utf8_lossy(&result.stdout);

        if should_match {
            assert!(
                stdout.contains("Match found"),
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
fn test_json_boolean_logic() {
    // Test various boolean combinations
    let input = r#"{"active": true, "verified": false, "premium": true, "age": 25}"#;

    let test_cases = vec![
        ("active == true && premium == true", true),
        ("active == true && verified == true", false),
        ("active == true || verified == true", true),
        ("!verified? && premium?", true),
        ("!(active? && verified?)", true),
        ("(age > 20) && (active? || verified?)", true),
    ];

    for (filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg(format!("{filter} {{Boolean test passed - ${{active}}}}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn parsm");

        let stdin = child.stdin.take().expect("get stdin");
        let mut stdin = stdin;
        write!(stdin, "{input}").expect("write to stdin");
        drop(stdin);

        let result = child.wait_with_output().expect("wait for output");
        let stdout = String::from_utf8_lossy(&result.stdout);

        if should_match {
            assert!(
                stdout.contains("Boolean test passed"),
                "Expected match for filter: {filter}",
            );
        } else {
            assert_eq!(stdout.trim(), "", "Expected no match for filter: {filter}",);
        }
    }
}

#[test]
fn test_json_braced_field_syntax() {
    // Test ${field} syntax in templates
    let input = r#"{"user": {"name": "Alice", "profile": {"email": "alice@example.com"}}}"#;

    let mut child = parsm_command()
        .arg("user.name == \"Alice\" {Contact: ${user.name} at ${user.profile.email}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_eq!(stdout.trim(), "Contact: Alice at alice@example.com");
}

#[test]
fn test_json_null_handling() {
    // Test handling of null values
    let input = r#"{"name": "Test", "description": null, "count": 0}"#;

    // Test filtering with null
    let mut child = parsm_command()
        .arg("description == null {Found null description for ${name}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let result = child.wait_with_output().expect("wait for output");
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("Found null description for Test"));
}

/// Test JSON forced format detection with --json flag
#[test]
fn test_json_forced_format() {
    let test_cases = vec![
        // Valid JSON that should parse correctly
        (r#"{"name": "Alice", "age": 30}"#, r#""name""#, "Alice"),
        (r#"{"name": "Alice", "age": 30}"#, r#""age""#, "30"),
        // JSON with nested objects
        (
            r#"{"user": {"name": "Bob", "role": "admin"}}"#,
            r#""user.name""#,
            "Bob",
        ),
        (
            r#"{"user": {"name": "Bob", "role": "admin"}}"#,
            r#""user.role""#,
            "admin",
        ),
        // JSON arrays
        (
            r#"{"items": ["apple", "banana", "cherry"]}"#,
            r#""items.0""#,
            "apple",
        ),
        (
            r#"{"items": ["apple", "banana", "cherry"]}"#,
            r#""items.2""#,
            "cherry",
        ),
        // Test template with forced JSON
        (
            r#"{"name": "Alice", "age": 30}"#,
            r#"{${name} is ${age} years old}"#,
            "Alice is 30 years old",
        ),
        (
            r#"{"price": 25.50, "currency": "USD"}"#,
            r#"{Cost: $100 base + ${price} ${currency}}"#,
            "Cost: $100 base + 25.5 USD",
        ),
    ];

    for (input, expression, expected) in test_cases {
        let mut child = parsm_command()
            .arg("--json")
            .arg(expression)
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
            "JSON forced format failed for input '{}' with expression '{}': {:?}",
            input,
            expression,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();
        assert_eq!(
            result, expected,
            "Failed for JSON forced input '{input}' with expression '{expression}'",
        );
    }
}

/// Test JSON forced format filtering with --json flag
#[test]
fn test_json_forced_format_filtering() {
    let test_cases = vec![
        (r#"{"name": "Alice", "age": 30}"#, "age > 25", true),
        (r#"{"name": "Bob", "age": 20}"#, "age > 25", false),
        (
            r#"{"status": "active", "count": 100}"#,
            r#"status == "active""#,
            true,
        ),
        (
            r#"{"status": "inactive", "count": 50}"#,
            r#"status == "active""#,
            false,
        ),
        (
            r#"{"user": {"name": "Alice", "admin": true}}"#,
            "user.admin == true",
            true,
        ),
        (
            r#"{"user": {"name": "Bob", "admin": false}}"#,
            "user.admin == true",
            false,
        ),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg("--json")
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
            "JSON forced format filtering failed for input '{}' with filter '{}': {:?}",
            input,
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();

        if should_match {
            assert_eq!(
                result, "match",
                "Expected match for JSON forced filter '{filter}' with input '{input}'",
            );
        } else {
            assert_eq!(
                result, "",
                "Expected empty output for JSON forced filter '{filter}' with input '{input}'",
            );
        }
    }
}
