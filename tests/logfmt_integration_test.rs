mod common;

use common::command;

#[test]
fn test_logfmt_field_selection() {
    // Test basic field selection from logfmt
    let input = r#"level=info msg="Starting application" service=api port=8080
level=error msg="Database connection failed" service=api code=500
level=warn msg="High memory usage" service=worker memory=85%"#;

    let mut cmd = command();
    cmd.arg("level");
    let output = common::run(cmd, input);

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

    let mut cmd = command();
    cmd.arg("user_name");
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("level == \"error\"");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("level=error"));
    assert!(lines[0].contains("msg=\"Request failed\""));
    assert!(lines[0].contains("status=500"));
}

#[test]
fn test_logfmt_template_rendering() {
    // Test template rendering with logfmt
    let input = r#"timestamp="2023-12-01T10:00:00Z" level=info msg="User login" user_id=123 username="alice"
timestamp="2023-12-01T10:01:00Z" level=warn msg="Failed login attempt" user_id=456 username="bob""#;

    let mut cmd = command();
    cmd.arg("{${level}: ${msg} - User: ${username}}");
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("status == \"200\" {$method request took $duration}");
    let output = common::run(cmd, input);

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

    let mut cmd = command();
    cmd.arg("msg");
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("[${level} ${endpoint} - ${response_time}ms (success: ${success})]");
    let output = common::run(cmd, input);

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

    let mut cmd = command();
    cmd.arg("service == \"api\" && status == \"200\"");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("method=GET"));
    assert!(lines[0].contains("status=200"));
    assert!(lines[0].contains("duration=100"));
}

#[test]
fn test_logfmt_dollar_template_syntax() {
    // Test $variable template syntax with logfmt
    let input = r#"level=info msg="Request processed" user=alice action=login timestamp=1701417600
level=warn msg="Rate limit exceeded" user=bob action=api_call timestamp=1701417661"#;

    let mut cmd = command();
    cmd.arg("{$user performed $action at $timestamp}");
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("msg");
    let output = common::run(cmd, input);

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

    let cmd = command();
    let output = common::run(cmd, input);
    assert!(output.status.success(), "parsm failed: {output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 1);

    // Convert mode writes the logfmt record as JSON, keys in input order
    assert_eq!(
        lines[0],
        r#"{"time":"2023-12-01T10:00:00Z","level":"info","msg":"Application started","version":"1.2.3"}"#
    );
}

#[test]
fn test_logfmt_error_handling() {
    // Test handling of malformed logfmt
    let input = r#"level=info msg="Good entry" status=200
malformed entry without equals
level=error msg="Another good entry" status=500"#;

    let mut cmd = command();
    cmd.arg("level");
    let output = common::run(cmd, input);

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
fn test_logfmt_large_dataset() {
    // A smoke test on a large input: every record makes it through.
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

    let mut cmd = command();
    cmd.arg("user_id");
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = common::stdout_of(&output);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 100);
    assert_eq!(lines[0], "1001");
    assert_eq!(lines[99], "1100");
}

/// Test logfmt forced format detection with --logfmt flag
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
            "user=alice action=login success=true",
            r#"{User ${user} ${action}: ${success}}"#,
            "User alice login: true",
        ),
    ];

    for (input, expression, expected) in test_cases {
        assert_logfmt_forced_format(input, expression, expected);
    }
}

/// Run a single `--logfmt` forced-format case and assert its rendered output.
fn assert_logfmt_forced_format(input: &str, expression: &str, expected: &str) {
    let mut cmd = command();
    cmd.arg("--logfmt").arg(expression);
    let output = common::run(cmd, input);
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

/// A `[...]` bracket span nested inside a `{...}` brace template renders
/// correctly, regardless of input format.
#[test]
fn logfmt_forced_format_nested_bracket_in_brace() {
    assert_logfmt_forced_format(
        r#"level=error msg="timeout" service=api"#,
        r#"{[${level}] ${msg} from ${service}}"#,
        "[error] timeout from api",
    );
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
        let mut cmd = command();
        cmd.arg("--logfmt").arg(filter).arg(r#"{match}"#);
        let output = common::run(cmd, input);
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

#[test]
fn test_logfmt_detection_requires_successful_parse() {
    // A stray '=' must not be enough to classify a line as logfmt: only a
    // line that parse_logfmt actually accepts stays logfmt. Text like
    // "x = 5 is the answer" falls through to plain text instead of failing.
    let test_cases = vec![
        ("x = 5 is the answer", "[${word_0}]", "x"),
        ("level=info started server", "[${word_1}]", "started"),
    ];

    for (input, expression, expected) in test_cases {
        let mut cmd = command();
        cmd.arg(expression);
        let output = common::run(cmd, input);
        assert!(
            output.status.success(),
            "parsm failed for input '{}': stderr={}",
            input,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.trim(),
            expected,
            "Failed for input '{input}' with expression '{expression}'",
        );
    }
}

#[test]
fn logfmt_filter_streams_first_record_before_eof() {
    let mut cmd = command();
    cmd.arg(r#"level == "error""#);
    let streamed = common::first_line_while_open(cmd, "level=error x=1", "level=info x=2");
    assert_eq!(streamed.first.as_deref(), Some("level=error x=1"));
    assert_eq!(streamed.rest, "");
    assert!(streamed.status.success());
}

#[test]
fn debug_log_names_detected_format_on_stderr() {
    let mut cmd = command();
    cmd.env("RUST_LOG", "parsm=debug").arg("a");
    let output = common::run(cmd, "a=1");
    assert_eq!(common::stdout_of(&output), "1\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("logfmt"), "stderr: {stderr}");
    assert!(output.status.success());
}

#[test]
fn warning_follows_the_records_before_it() {
    let mut cmd = command();
    cmd.arg("a");
    let output = common::run_merged(cmd, "a=1\nbad line\na=2\n");
    assert_eq!(
        output,
        "1\nWarning: failed to parse line 2: expected key=value pairs\n2\n"
    );
}

// Ported from the retired Python integration harness; each asserts the
// same input, arguments and expected stdout as its original case (see
// the batch b6 REPORT's mapping table).

#[test]
fn ported_logfmt_field_select() {
    let mut cmd = command();
    cmd.arg("level");
    let output = common::run(cmd, r#"level=info msg="User login" user_id=123"#);
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "info");
}

#[test]
fn ported_logfmt_quoted_value() {
    let mut cmd = command();
    cmd.arg("user");
    let output = common::run(cmd, r#"level=info msg="User login" user="Alice Smith""#);
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice Smith");
}

#[test]
fn ported_logfmt_filter() {
    let mut cmd = command();
    cmd.arg(r#"level == "error""#);
    let output = common::run(cmd, r#"level=error msg="Database error" service=api"#);
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(
        common::stdout_of(&output).trim(),
        r#"level=error msg="Database error" service=api"#
    );
}

#[test]
fn ported_logfmt_template() {
    let mut cmd = command();
    cmd.arg("{[${level}] ${msg}}");
    let output = common::run(cmd, r#"level=error msg="DB error" service=api"#);
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "[error] DB error");
}

#[test]
fn ported_logfmt_numeric() {
    let mut cmd = command();
    cmd.arg("response_time");
    let output = common::run(cmd, "level=info response_time=250 status=200");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "250");
}

#[test]
fn ported_detect_logfmt() {
    let mut cmd = command();
    cmd.arg("format");
    let output = common::run(cmd, "format=logfmt level=info");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "logfmt");
}

#[test]
fn ported_truthy_logfmt_present() {
    let mut cmd = command();
    cmd.arg("active?");
    let output = common::run(cmd, "active=true name=Alice");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "active=true name=Alice");
}

#[test]
fn ported_explicit_logfmt() {
    let mut cmd = command();
    cmd.args(["--logfmt", "name"]);
    let output = common::run(cmd, "name=Alice age=30");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice");
}
/// Kitchen sink: field access, a case-insensitive regex, `&&`/`||`/
/// `!field?`, and a template conditional plus `${0}`, combined in one
/// expression and asserted against exact output.
#[test]
fn logfmt_kitchen_sink() {
    let input = "name=Alice email=alice@EXAMPLE.com active=true banned=false role=admin";
    let mut cmd = command();
    cmd.args([
        "--logfmt",
        r#"(email ~= /example\.com/i || role == "admin") && !banned? {Role: ${role} - Active: ${active?yes:no} - Source: ${0}}"#,
    ]);
    let output = common::run(cmd, input);
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(
        common::stdout_of(&output).trim_end_matches('\n'),
        format!("Role: admin - Active: yes - Source: {input}")
    );
}
