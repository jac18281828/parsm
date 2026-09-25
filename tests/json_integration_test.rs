use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

mod common;

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
    // A guessed JSON value that fails before its first record falls through
    // the precedence table; this input is read as text lines.
    let input = r#"{"name": "Invalid JSON"
{"name": "Second line", "age": 25}"#;
    let mut cmd = parsm_command();
    cmd.arg("[${word_1}]");
    let output = common::run(cmd, input);
    assert_eq!(stdout_of(&output), "\"Invalid\n\"Second\n");
    assert!(output.status.success());

    // Forced JSON never falls through: the first-record failure is fatal.
    let mut cmd = parsm_command();
    cmd.args(["--json", "name"]);
    let output = common::run(cmd, input);
    assert_eq!(stdout_of(&output), "");
    assert_eq!(output.status.code(), Some(1));

    // After the first record, a failure warns and the stream resumes at the
    // next line.
    let mut cmd = parsm_command();
    cmd.arg("name");
    let output = common::run(
        cmd,
        "{\"name\": \"First\"}\n{\"name\": \"Broken\"\n{\"name\": \"Third\"}",
    );
    assert_eq!(stdout_of(&output), "First\nThird\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("Warning: failed to parse line 2: "),
        "stderr: {stderr}"
    );
    assert!(output.status.success());
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

#[test]
fn test_json_lines_filter() {
    // Multi-document JSON (JSON Lines) input through a bare filter must not be
    // misdetected as headered CSV and silently drop all output.
    let input = "{\"id\": 1, \"name\": \"Alice\"}\n{\"id\": 2, \"name\": \"Bob\"}\n{\"id\": 3, \"name\": \"Charlie\"}\n";

    let mut child = parsm_command()
        .arg("id > 1")
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
    assert!(stdout.contains("\"name\": \"Bob\""), "stdout: {stdout}");
    assert!(stdout.contains("\"name\": \"Charlie\""), "stdout: {stdout}");
    assert!(!stdout.contains("\"name\": \"Alice\""), "stdout: {stdout}");
}

#[test]
fn test_json_multibyte_detection_prefix() {
    // The 100-byte detection prefix must land on a char boundary. Byte 100
    // of this line falls inside the two-byte UTF-8 encoding of 'é'.
    let input = format!("{{\"a\":\"{}{}\"}}", "x".repeat(93), 'é');

    let mut child = parsm_command()
        .arg("a")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.as_mut().expect("get stdin");
    stdin.write_all(input.as_bytes()).expect("write to stdin");
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("wait for parsm");
    assert!(
        output.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), format!("{}{}", "x".repeat(93), 'é'));
}

#[test]
fn test_invalid_rust_log_falls_back_to_warn() {
    // A malformed RUST_LOG must not abort the process; parsm falls back to
    // its default filter and continues processing.
    let mut child = parsm_command()
        .env("RUST_LOG", "parsm=bogus[")
        .arg("a == 1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.as_mut().expect("get stdin");
    stdin.write_all(b"{\"a\": 1}\n").expect("write to stdin");
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("wait for parsm");
    assert!(
        output.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "{\"a\": 1}");
}

#[test]
fn test_json_lines_field_selector() {
    // Multi-document JSON (JSON Lines) input through a field selector must
    // extract from each document in order, not be dropped by CSV misdetection.
    let input = "{\"id\": 1, \"name\": \"Alice\"}\n{\"id\": 2, \"name\": \"Bob\"}\n{\"id\": 3, \"name\": \"Charlie\"}\n";

    let mut child = parsm_command()
        .arg("name")
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
    assert_eq!(stdout.trim(), "Alice\nBob\nCharlie");
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn json_filter_streams_first_record_before_eof() {
    let mut cmd = parsm_command();
    cmd.arg("a > 1");
    let streamed = common::first_line_while_open(cmd, r#"{"a":2}"#, r#"{"a":3}"#);
    assert_eq!(streamed.first.as_deref(), Some(r#"{"a":2}"#));
    assert_eq!(streamed.rest, "{\"a\":3}\n");
    assert!(streamed.status.success());
}

#[test]
fn json_array_items_carry_their_own_source() {
    let mut cmd = parsm_command();
    cmd.arg("a > 3");
    let output = common::run(cmd, r#"[{"a":1},{"a":5},{"a":7}]"#);
    assert_eq!(stdout_of(&output), "{\"a\":5}\n{\"a\":7}\n");
    assert!(output.status.success());
}

#[test]
fn json_pretty_values_are_one_record_each() {
    let mut cmd = parsm_command();
    cmd.arg("a");
    let output = common::run(cmd, "{\n \"a\": 1\n}\n{\n \"a\": 5\n}\n");
    assert_eq!(stdout_of(&output), "1\n5\n");
    assert!(output.status.success());
}

#[test]
fn json_lines_and_pretty_values_mix() {
    let mut cmd = parsm_command();
    cmd.arg("a");
    let output = common::run(cmd, "{\"a\":1}\n{\n \"a\":5\n}\n");
    assert_eq!(stdout_of(&output), "1\n5\n");
    assert!(output.status.success());
}

#[test]
fn forced_json_rejects_text_input() {
    let mut cmd = parsm_command();
    cmd.args(["--json", r#"word_0 == "hello""#]);
    let output = common::run(cmd, "hello world");
    assert_eq!(stdout_of(&output), "");
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn json_convert_keeps_key_order_per_item() {
    let output = common::run(parsm_command(), r#"[{"b":1,"a":2},{"c":3}]"#);
    assert_eq!(stdout_of(&output), "{\"b\":1,\"a\":2}\n{\"c\":3}\n");
    assert!(output.status.success());
}

#[test]
fn json_scalar_exposes_original_input() {
    let mut cmd = parsm_command();
    cmd.arg("[${0}]");
    let output = common::run(cmd, "42");
    assert_eq!(stdout_of(&output), "42\n");
    assert!(output.status.success());
}

#[test]
fn json_values_sharing_a_line_are_separate_records() {
    let mut cmd = parsm_command();
    cmd.arg("a");
    let output = common::run(cmd, r#"{"a":1} {"a":2}"#);
    assert_eq!(stdout_of(&output), "1\n2\n");
    assert!(output.status.success());
}
