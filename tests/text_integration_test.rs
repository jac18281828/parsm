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
fn test_text_field_selection() {
    // Test basic field selection from text (word selection by index)
    let input = "hello world test\nquick brown fox\njumps over lazy";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("word_0") // First word
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "hello");
    assert_eq!(lines[1], "quick");
    assert_eq!(lines[2], "jumps");
}

#[test]
fn test_text_array_access() {
    // Test accessing text as an array with numeric indices
    let input = "alpha beta gamma delta\none two three four";

    let mut child = parsm_command()
        .arg("word_2") // Third element (zero-indexed)
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
    assert_eq!(lines[0], "gamma");
    assert_eq!(lines[1], "three");
}

#[test]
fn test_text_template_rendering() {
    // Test template rendering with text data
    let input = "John 30 Engineer\nJane 25 Designer\nBob 35 Manager";

    let mut child = parsm_command()
        .arg("{$word_0 $word_1 $word_2}")
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

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "John 30 Engineer");
    assert_eq!(lines[1], "Jane 25 Designer");
    assert_eq!(lines[2], "Bob 35 Manager");
}

#[test]
fn test_text_dollar_template_syntax() {
    // Test $variable template syntax with text
    let input = "error connection timeout\ninfo server started\nwarn high memory";

    let mut child = parsm_command()
        .arg("{$word_0 $word_1 $word_2}")
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

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "error connection timeout");
    assert_eq!(lines[1], "info server started");
    assert_eq!(lines[2], "warn high memory");
}

#[test]
fn test_text_filter_operations() {
    // Test filtering text entries
    let input =
        "error connection failed\ninfo server started\nerror database timeout\nwarn memory high";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Test filtering by first word
    let output = parsm_command()
        .arg("word_0 == \"error\"")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("error connection failed"));
    assert!(lines[1].contains("error database timeout"));
}

#[test]
fn test_text_filter_with_template() {
    // Test combined filter and template with text
    let input = "error connection failed retry\ninfo server started successfully\nerror database timeout critical\nwarn memory high alert";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Filter error entries and format them
    let output = parsm_command()
        .arg("word_0 == \"error\" {ERROR: $word_1 $word_2}")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "ERROR: connection failed");
    assert_eq!(lines[1], "ERROR: database timeout");
}

#[test]
fn test_text_format_detection() {
    // Test that text is correctly detected and parsed
    let input = "this is plain text without special formatting";

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

    // Now we expect the original input as output instead of JSON
    assert_eq!(lines[0], input.trim());
}

#[test]
fn test_text_empty_lines() {
    // Test handling of empty lines and whitespace
    let input = "hello world\n\n   \nfoo bar\n";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("word_0")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // Empty lines should be skipped
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "hello");
    assert_eq!(lines[1], "foo");
}

#[test]
fn test_text_single_word() {
    // Test handling of lines with a single word
    let input = "hello\nworld\ntest";

    let mut child = parsm_command()
        .arg("word_0")
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

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "hello");
    assert_eq!(lines[1], "world");
    assert_eq!(lines[2], "test");
}

#[test]
fn test_text_multiple_spaces() {
    // Test handling of multiple spaces between words
    let input = "word1    word2     word3\nalpha  beta   gamma";

    let mut child = parsm_command()
        .arg("{$word_0-$word_1-$word_2}")
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
    assert_eq!(lines[0], "word1-word2-word3");
    assert_eq!(lines[1], "alpha-beta-gamma");
}

#[test]
fn test_text_nonexistent_field() {
    // Test accessing non-existent fields
    let input = "one two\nthree four";

    let mut child = parsm_command()
        .arg("word_5") // Word that doesn't exist
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

    // Should produce warnings but not crash
    assert!(stdout.trim().is_empty() || !output.stderr.is_empty());
}

#[test]
fn test_text_original_input_template() {
    // Test accessing the original input line
    let input = "first line of text\nsecond line here";

    let mut child = parsm_command()
        .arg("{${0}}")
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
    assert_eq!(lines[0], "first line of text");
    assert_eq!(lines[1], "second line here");
}

#[test]
fn test_text_complex_filtering() {
    // Test complex filtering with logical operators
    let input = "error network timeout critical\ninfo server started normal\nerror disk full critical\nwarn memory high normal\ninfo backup completed normal";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Filter for errors that are critical
    let output = parsm_command()
        .arg("word_0 == \"error\" && word_3 == \"critical\"")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("error network timeout critical"));
    assert!(lines[1].contains("error disk full critical"));
}

#[test]
fn test_text_error_handling() {
    // Test handling of malformed input that doesn't prevent processing
    let input = "good line one\n\nbad line with weird characters: @#$%\ngood line two";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let output = parsm_command()
        .arg("word_0")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    // Should succeed and process all lines (text format is very permissive)
    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    // Should have output from all non-empty lines
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "good");
    assert_eq!(lines[1], "bad");
    assert_eq!(lines[2], "good");
}

#[test]
fn test_text_performance_large_dataset() {
    // Test performance with a larger text dataset
    let mut input = String::new();
    for i in 1..=100 {
        input.push_str(&format!("entry {i} processing data item number {i}\n"));
    }

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    let start_time = std::time::Instant::now();
    let output = parsm_command()
        .arg("word_1") // Second word
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");
    let duration = start_time.elapsed();

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 100);
    assert_eq!(lines[0], "1");
    assert_eq!(lines[99], "100");

    // Should process reasonably quickly
    assert!(
        duration.as_millis() < 1000,
        "Processing took too long: {duration:?}"
    );
}

#[test]
fn test_text_string_operations() {
    // Test string operations like contains
    let input = "user alice logged in\nuser bob failed login\nuser charlie logged out\nadmin alice system check";

    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "{input}").expect("write temp file");

    // Filter for lines containing "alice"
    let output = parsm_command()
        .arg("word_1 == \"alice\"")
        .stdin(File::open(file.path()).unwrap())
        .output()
        .expect("run parsm");

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("user alice logged in"));
    assert!(lines[1].contains("admin alice system check"));
}

#[test]
fn test_text_mixed_content() {
    // Test text format with varied content (numbers, punctuation, etc.)
    let input = "Item-123 Processing at 14.30.45\nWarning Memory usage 85%\nStatus=OK Count=42";

    let mut child = parsm_command()
        .arg("{$word_0 -> $word_1}")
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

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Item-123 -> Processing");
    assert_eq!(lines[1], "Warning -> Memory");
    assert_eq!(lines[2], "Status=OK -> Count=42");
}

/// Test text forced format parsing with --text flag
#[test]
fn test_text_forced_format() {
    let test_cases = vec![
        // Space-separated text
        ("Alice 30 Engineer", r#""word_0""#, "Alice"),
        ("Alice 30 Engineer", r#""word_1""#, "30"),
        ("Alice 30 Engineer", r#""word_2""#, "Engineer"),
        // Text that might look like other formats but forced as text
        ("hello world", r#""word_0""#, "hello"),
        ("hello world", r#""word_1""#, "world"),
        // Text with colons that might be mistaken for other formats
        ("name: Alice age: 30", r#""word_0""#, "name:"),
        ("name: Alice age: 30", r#""word_1""#, "Alice"),
        ("name: Alice age: 30", r#""word_2""#, "age:"),
        ("name: Alice age: 30", r#""word_3""#, "30"),
        // Test template with forced text
        (
            "Alice 30 Engineer",
            r#"{Name: ${word_0}, Age: ${word_1}}"#,
            "Name: Alice, Age: 30",
        ),
        (
            "Hello world test",
            r#"{${word_0} ${word_2}!}"#,
            "Hello test!",
        ),
        (
            "name: Alice age: 30",
            r#"{${word_1} is ${word_3} years old}"#,
            "Alice is 30 years old",
        ),
    ];

    for (input, expression, expected) in test_cases {
        let mut child = parsm_command()
            .arg("--text")
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
            "Text forced format failed for input '{}' with expression '{}': {:?}",
            input,
            expression,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();
        assert_eq!(
            result, expected,
            "Failed for text forced input '{input}' with expression '{expression}'",
        );
    }
}

/// Test text forced format filtering with --text flag
#[test]
fn test_text_forced_format_filtering() {
    let test_cases = vec![
        ("Alice 30 Engineer", r#"word_0 == "Alice""#, true),
        ("Bob 25 Student", r#"word_0 == "Alice""#, false),
        ("Alice 30 Engineer", r#"word_2 *= "Eng""#, true),
        ("Bob 25 Student", r#"word_2 *= "Eng""#, false),
        ("count 100 items", r#"word_1 == "100""#, true),
        ("count 50 items", r#"word_1 == "100""#, false),
    ];

    for (input, filter, should_match) in test_cases {
        let mut child = parsm_command()
            .arg("--text")
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
            "Text forced format filtering failed for input '{}' with filter '{}': {:?}",
            input,
            filter,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.trim();

        if should_match {
            assert_eq!(
                result, "match",
                "Expected match for text forced filter '{filter}' with input '{input}'",
            );
        } else {
            assert_eq!(
                result, "",
                "Expected empty output for text forced filter '{filter}' with input '{input}'",
            );
        }
    }
}
