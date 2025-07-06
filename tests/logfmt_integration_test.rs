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
fn test_logfmt_field_selection() {
    // Test basic field selection from logfmt
    let input = r#"level=info msg="Starting application" service=api port=8080
level=error msg="Database connection failed" service=api code=500
level=warn msg="High memory usage" service=worker memory=85%"#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("level")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "info");
    assert_eq!(lines[1], "error");
    assert_eq!(lines[2], "warn");
}

#[test]
fn test_logfmt_nested_field_selection() {
    // Test field selection from logfmt with flat field names (using underscores instead of dots)
    let input = r#"timestamp="2023-12-01T10:00:00Z" level=info msg="User login" user_id=123 user_name="Alice"
timestamp="2023-12-01T10:01:00Z" level=info msg="User logout" user_id=456 user_name="Bob""#;

    let mut child = parsm_command()
        .arg("user_name")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("wait for parsm");
    assert!(output.status.success(), "parsm failed: {output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Alice");
    assert_eq!(lines[1], "Bob");
}

#[test]
fn test_logfmt_filter_operations() {
    // Test filtering logfmt entries
    let input = r#"level=info msg="Request started" duration=250ms status=200
level=error msg="Request failed" duration=5000ms status=500
level=info msg="Request completed" duration=100ms status=200
level=warn msg="Slow request" duration=3000ms status=200"#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Test filtering by level
    let output = parsm_command()
        .arg("level == \"error\"")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("\"level\":\"error\""));
    assert!(lines[0].contains("\"msg\":\"Request failed\""));
    assert!(lines[0].contains("\"status\":\"500\""));
}

#[test]
fn test_logfmt_template_rendering() {
    // Test template rendering with logfmt
    let input = r#"timestamp="2023-12-01T10:00:00Z" level=info msg="User login" user_id=123 username="alice"
timestamp="2023-12-01T10:01:00Z" level=warn msg="Failed login attempt" user_id=456 username="bob""#;

    let mut child = parsm_command()
        .arg("{${level}: ${msg} - User: ${username}}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("wait for parsm");
    assert!(output.status.success(), "parsm failed: {output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "info: User login - User: alice");
    assert_eq!(lines[1], "warn: Failed login attempt - User: bob");
}

#[test]
fn test_logfmt_filter_with_template() {
    // Test combined filter and template with logfmt
    let input = r#"level=info msg="Request" method=GET status=200 duration=120ms
level=info msg="Request" method=POST status=201 duration=350ms
level=error msg="Request" method=GET status=500 duration=5000ms
level=info msg="Request" method=PUT status=200 duration=200ms"#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Filter successful requests and format them
    let output = parsm_command()
        .arg("status == \"200\" {$method request took $duration}")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "GET request took 120ms");
    assert_eq!(lines[1], "PUT request took 200ms");
}

#[test]
fn test_logfmt_escaped_quotes() {
    // Test logfmt with escaped quotes (simplified case)
    let input = r#"level=info msg=\"Server starting on port 8080\" config=\"/etc/app.conf\"
level=error msg=\"Failed to load config file\" error=\"not found\""#;

    let mut child = parsm_command()
        .arg("msg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("wait for parsm");
    assert!(output.status.success(), "parsm failed: {output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Server starting on port 8080");
    assert_eq!(lines[1], "Failed to load config file");
}

#[test]
fn test_logfmt_mixed_value_types() {
    // Test logfmt with different value types (quoted, unquoted, numbers)
    let input = r#"timestamp=1701417600 level=info msg="API request" endpoint="/users" response_time=250 success=true user_id=12345
timestamp=1701417661 level=error msg="Database error" endpoint="/orders" response_time=5000 success=false error_code=DB_TIMEOUT"#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Test template with mixed value types
    let output = parsm_command()
        .arg("[${level} ${endpoint} - ${response_time}ms (success: ${success})]")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "info /users - 250ms (success: true)");
    assert_eq!(lines[1], "error /orders - 5000ms (success: false)");
}

#[test]
fn test_logfmt_complex_filtering() {
    // Test complex filtering with logical operators
    let input = r#"level=info service=api method=GET status=200 duration=100
level=info service=api method=POST status=201 duration=300
level=error service=api method=GET status=500 duration=5000
level=info service=worker method=PUT status=200 duration=150
level=warn service=worker method=DELETE status=404 duration=200"#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Filter for API service with successful status codes
    let output = parsm_command()
        .arg("service == \"api\" && status == \"200\"")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("\"method\":\"GET\""));
    assert!(lines[0].contains("\"status\":\"200\""));
    assert!(lines[0].contains("\"duration\":\"100\""));
}

#[test]
fn test_logfmt_dollar_template_syntax() {
    // Test $variable template syntax with logfmt
    let input = r#"level=info msg="Request processed" user=alice action=login timestamp=1701417600
level=warn msg="Rate limit exceeded" user=bob action=api_call timestamp=1701417661"#;

    let mut child = parsm_command()
        .arg("{$user performed $action at $timestamp}")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("wait for parsm");
    assert!(output.status.success(), "parsm failed: {output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "alice performed login at 1701417600");
    assert_eq!(lines[1], "bob performed api_call at 1701417661");
}

#[test]
fn test_logfmt_empty_and_special_values() {
    // Test logfmt with empty values and special characters
    let input = r#"level=info msg="" status=200 user_agent="Mozilla/5.0" referer=""
level=debug msg="Special chars: []{}()" path="/api/v1/test" query="?param=value&other=test""#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Test field selection on empty values
    let output = parsm_command()
        .arg("msg")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.split('\n').collect();
    // Remove only the last empty line if it exists (from final newline)
    let lines: Vec<&str> = if lines.last() == Some(&"") {
        lines[..lines.len() - 1].to_vec()
    } else {
        lines
    };

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], ""); // Empty string
    assert_eq!(lines[1], "Special chars: []{}()");
}

#[test]
fn test_logfmt_format_detection() {
    // Test that logfmt is correctly detected and parsed
    let input = r#"time=2023-12-01T10:00:00Z level=info msg="Application started" version=1.2.3"#;

    let mut child = parsm_command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let stdin = child.stdin.take().expect("get stdin");
    let mut stdin = stdin;
    write!(stdin, "{input}").expect("write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("wait for parsm");
    assert!(output.status.success(), "parsm failed: {output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 1);

    // Verify it's parsed as JSON (logfmt gets converted to JSON)
    let parsed: serde_json::Value = serde_json::from_str(lines[0]).expect("parse JSON");
    assert_eq!(parsed["time"], "2023-12-01T10:00:00Z");
    assert_eq!(parsed["level"], "info");
    assert_eq!(parsed["msg"], "Application started");
    assert_eq!(parsed["version"], "1.2.3");
}

#[test]
fn test_logfmt_error_handling() {
    // Test handling of malformed logfmt
    let input = r#"level=info msg="Good entry" status=200
malformed entry without equals
level=error msg="Another good entry" status=500"#;

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("level")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    // Should succeed but only process valid logfmt lines
    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // Should have output from the two valid logfmt lines
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "info");
    assert_eq!(lines[1], "error");
}

#[test]
fn test_logfmt_performance_large_dataset() {
    // Test performance with a larger logfmt dataset
    let mut input = String::new();
    for i in 1..=100 {
        input.push_str(&format!(
            "timestamp={timestamp} level=info msg=\"Processing request {i}\" user_id={user_id} duration={duration}ms\n",
            timestamp = 1701417600 + i,
            i = i,
            user_id = 1000 + i,
            duration = 50 + (i % 200)
        ));
    }

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let start_time = std::time::Instant::now();
    let output = parsm_command()
        .arg("user_id")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");
    let duration = start_time.elapsed();

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 100);
    assert_eq!(lines[0], "1001");
    assert_eq!(lines[99], "1100");

    // Should process reasonably quickly
    assert!(
        duration.as_millis() < 1000,
        "Processing took too long: {duration:?}"
    );
}

/// Test logfmt forced format parsing with --logfmt flag
#[test]
fn test_logfmt_forced_format() {
    let test_cases = vec![
        // Standard logfmt parsing
        (
            "level=error msg=\"DB error\" service=api",
            r#""level""#,
            "error",
        ),
        (
            "level=error msg=\"DB error\" service=api",
            r#""msg""#,
            "DB error",
        ),
        (
            "level=error msg=\"DB error\" service=api",
            r#""service""#,
            "api",
        ),
        // Logfmt with numeric values
        (
            "timestamp=1234567890 level=info count=42",
            r#""timestamp""#,
            "1234567890",
        ),
        (
            "timestamp=1234567890 level=info count=42",
            r#""count""#,
            "42",
        ),
        // Test template with forced logfmt
        (
            "level=error msg=\"timeout\" service=api",
            r#"{[${level}] ${msg} from ${service}}"#,
            "[error] timeout from api",
        ),
        (
            "user=alice action=login success=true",
            r#"{User ${user} ${action}: ${success}}"#,
            "User alice login: true",
        ),
    ];

    for (input, expression, expected) in test_cases {
        let mut child = parsm_command()
            .arg("--logfmt")
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
            "Logfmt forced format failed for input '{}' with expression '{}': {:?}",
            input,
            expression,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();
        assert_eq!(
            result, expected,
            "Failed for logfmt forced input '{input}' with expression '{expression}'",
        );
    }
}

/// Test logfmt forced format filtering with --logfmt flag
#[test]
fn test_logfmt_forced_format_filtering() {
    let test_cases = vec![
        (
            "level=error msg=\"DB error\" service=api",
            r#"level == "error""#,
            true,
        ),
        (
            "level=info msg=\"startup\" service=api",
            r#"level == "error""#,
            false,
        ),
        (
            "user=alice role=admin active=true",
            r#"active == "true""#,
            true,
        ),
        (
            "user=bob role=user active=false",
            r#"active == "true""#,
            false,
        ),
        ("count=100 threshold=50", r#"count == "100""#, true),
        ("count=25 threshold=50", r#"count == "100""#, false),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg("--logfmt")
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
            "Logfmt forced format filtering failed for input '{}' with filter '{}': {:?}",
            input,
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();

        if should_match {
            assert_eq!(
                result, "match",
                "Expected match for logfmt forced filter '{filter}' with input '{input}'",
            );
        } else {
            assert_eq!(
                result, "",
                "Expected empty output for logfmt forced filter '{filter}' with input '{input}'",
            );
        }
    }
}
