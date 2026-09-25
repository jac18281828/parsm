mod common;

use common::command;

#[test]
fn test_text_field_selection() {
    // Test basic field selection from text (word selection by index)
    let input = "hello world test\nquick brown fox\njumps over lazy";

    let mut cmd = command();
    cmd.arg("word_0");
    let output = common::run(cmd, input);

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

    let mut cmd = command();
    cmd.arg("word_2"); // Third element (zero-indexed)
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("{$word_0 $word_1 $word_2}");
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("{$word_0 $word_1 $word_2}");
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("word_0 == \"error\"");
    let output = common::run(cmd, input);

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

    let mut cmd = command();
    cmd.arg("word_0 == \"error\" {ERROR: $word_1 $word_2}");
    let output = common::run(cmd, input);

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

    let cmd = command();
    let output = common::run(cmd, input);
    assert!(output.status.success(), "parsm failed: {output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 1);

    // Convert mode writes the text record as an array of words
    assert_eq!(
        lines[0],
        r#"["this","is","plain","text","without","special","formatting"]"#
    );
}

#[test]
fn test_text_empty_lines() {
    // Test handling of empty lines and whitespace
    let input = "hello world\n\n   \nfoo bar\n";

    let mut cmd = command();
    cmd.arg("word_0");
    let output = common::run(cmd, input);

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

    let mut cmd = command();
    cmd.arg("word_0");
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("{$word_0-$word_1-$word_2}");
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("word_5");
    let output = common::run(cmd, input);
    assert!(output.status.success(), "parsm failed: {output:?}");

    // A record without the field prints nothing and warns nothing
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn test_text_original_input_template() {
    // Test accessing the original input line
    let input = "first line of text\nsecond line here";

    let mut cmd = command();
    cmd.arg("{${0}}");
    let output = common::run(cmd, input);
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

    let mut cmd = command();
    cmd.arg("word_0 == \"error\" && word_3 == \"critical\"");
    let output = common::run(cmd, input);

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

    let mut cmd = command();
    cmd.arg("word_0");
    let output = common::run(cmd, input);

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
fn test_text_large_dataset() {
    // A smoke test on a large input: every record makes it through.
    let mut input = String::new();
    for i in 1..=100 {
        input.push_str(&format!("entry {i} processing data item number {i}\n"));
    }

    let mut cmd = command();
    cmd.arg("word_1"); // Second word
    let output = common::run(cmd, input);

    assert!(output.status.success(), "parsm failed: {output:?}");
    let stdout = common::stdout_of(&output);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 100);
    assert_eq!(lines[0], "1");
    assert_eq!(lines[99], "100");
}

#[test]
fn test_text_string_operations() {
    // Test string operations like contains
    let input = "user alice logged in\nuser bob failed login\nuser charlie logged out\nadmin alice system check";

    let mut cmd = command();
    cmd.arg("word_1 == \"alice\"");
    let output = common::run(cmd, input);

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

    let mut cmd = command();
    cmd.arg("{$word_0 -> $word_1}");
    let output = common::run(cmd, input);
    assert!(output.status.success(), "parsm failed: {output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Item-123 -> Processing");
    assert_eq!(lines[1], "Warning -> Memory");
    assert_eq!(lines[2], "Status=OK -> Count=42");
}

#[test]
fn test_text_multibyte_detection_prefix() {
    // The 100-byte detection prefix must land on a char boundary. Byte 100
    // of this line falls inside the two-byte UTF-8 encoding of 'é'.
    let first_word = format!("{}{}", "x".repeat(99), 'é');
    let input = format!("{first_word} tail");

    let mut cmd = command();
    cmd.arg("[${word_0}]");
    let output = common::run(cmd, input);
    assert!(
        output.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = common::stdout_of(&output);
    assert_eq!(stdout.trim(), first_word);
}

/// Test text forced format detection with --text flag
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
        let mut cmd = command();
        cmd.arg("--text").arg(expression);
        let output = common::run(cmd, input);
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
        let mut cmd = command();
        cmd.arg("--text").arg(filter).arg(r#"{match}"#);
        let output = common::run(cmd, input);
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

#[test]
fn text_template_streams_first_record_before_eof() {
    let mut cmd = command();
    cmd.arg("[${word_0}]");
    let streamed = common::first_line_while_open(cmd, "hello world", "bye now");
    assert_eq!(streamed.first.as_deref(), Some("hello"));
    assert_eq!(streamed.rest, "bye\n");
    assert!(streamed.status.success());
}

#[test]
fn forced_text_reads_json_as_words() {
    let mut cmd = command();
    cmd.args(["--text", "[${word_0}]"]);
    let output = common::run(cmd, r#"{"a":1}"#);
    assert_eq!(common::stdout_of(&output), "{\"a\":1}\n");
    assert!(output.status.success());
}

#[test]
fn text_convert_writes_word_arrays() {
    let output = common::run(command(), "hello world");
    assert_eq!(common::stdout_of(&output), "[\"hello\",\"world\"]\n");
    assert!(output.status.success());
}

#[test]
fn text_exposes_1_based_positional_fields() {
    let mut cmd = command();
    cmd.arg("[${1}-${2}]");
    let output = common::run(cmd, "Alice 30");
    assert_eq!(common::stdout_of(&output), "Alice-30\n");
    assert!(output.status.success());
}

#[test]
fn missing_field_prints_nothing() {
    let mut cmd = command();
    cmd.arg("nope");
    let output = common::run(cmd, "x y");
    assert_eq!(common::stdout_of(&output), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn bracketed_word_is_text() {
    let mut cmd = command();
    cmd.arg("[${0}]");
    let output = common::run(cmd, "[a]");
    assert_eq!(common::stdout_of(&output), "[a]\n");
    assert!(output.status.success());

    let output = common::run(command(), "[a]");
    assert_eq!(common::stdout_of(&output), "[\"[a]\"]\n");
}

#[test]
fn leading_number_with_trailing_words_is_text() {
    let mut cmd = command();
    cmd.arg("[${word_1}]");
    let output = common::run(cmd, "200 OK");
    assert_eq!(common::stdout_of(&output), "OK\n");
    assert!(output.status.success());
}

#[test]
fn text_first_line_with_unmatched_opener_streams() {
    let mut cmd = command();
    cmd.arg("[${word_1}]");
    let mut session = common::Session::start(cmd);
    session.write_line("2024 ERROR [worker {id=3");
    assert_eq!(session.read_lines(1), vec!["ERROR"]);
    session.write_line("2024 INFO done");
    assert_eq!(session.read_lines(1), vec!["INFO"]);
    let (rest, status) = session.finish();
    assert!(rest.is_empty(), "rest: {rest:?}");
    assert!(status.success());
}

#[test]
fn text_undecodable_later_line_warns_and_is_skipped() {
    let mut cmd = command();
    cmd.arg("[${word_0}]");
    let output = common::run(cmd, b"hello world\n\xff\xfe\nbye now\n");
    assert_eq!(common::stdout_of(&output), "hello\nbye\n");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Warning: failed to parse line 2: invalid UTF-8\n"
    );
    assert!(output.status.success());
}

#[test]
fn text_undecodable_first_line_is_fatal() {
    let mut cmd = command();
    cmd.arg("[${word_0}]");
    let output = common::run(cmd, b"\xff\xfe\nhello world\n");
    assert_eq!(common::stdout_of(&output), "");
    assert_eq!(output.status.code(), Some(1));
}

// Ported from the retired Python integration harness; each asserts the
// same input, arguments and expected stdout as its original case (see
// the batch b6 REPORT's mapping table).

#[test]
fn ported_text_word_select() {
    let mut cmd = command();
    cmd.arg("word_0");
    let output = common::run(cmd, "Alice 30 Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice");
}

#[test]
fn ported_text_word_template() {
    let mut cmd = command();
    cmd.arg("{${word_0} is ${word_1}}");
    let output = common::run(cmd, "Alice 30 Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice is 30");
}

#[test]
fn ported_text_multiword() {
    let mut cmd = command();
    cmd.arg("word_2");
    let output = common::run(cmd, "Hello world from parsm");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "from");
}

#[test]
fn ported_text_filter() {
    let mut cmd = command();
    cmd.arg(r#"word_1 > "25""#);
    let output = common::run(cmd, "Alice 30 Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice 30 Engineer");
}

#[test]
fn ported_unicode_text() {
    let mut cmd = command();
    cmd.arg("word_0");
    let output = common::run(cmd, "café 123 français");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "café");
}

#[test]
fn ported_explicit_text() {
    let mut cmd = command();
    cmd.args(["--text", "word_0"]);
    let output = common::run(cmd, "Alice 30 Engineer");
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(common::stdout_of(&output).trim(), "Alice");
}
/// Kitchen sink: positional word access, a case-insensitive regex, `&&`/
/// `||`/`!field?`, and a template conditional plus `${0}`, combined in one
/// expression and asserted against exact output.
#[test]
fn text_kitchen_sink() {
    let input = "Alice engineer alice@EXAMPLE.com true false";
    let mut cmd = command();
    cmd.args([
        "--text",
        r#"(word_2 ~= /example\.com/i || word_1 == "engineer") && !word_4? {Role: ${word_1} - Active: ${word_3?yes:no} - Source: ${0}}"#,
    ]);
    let output = common::run(cmd, input);
    assert!(output.status.success(), "parsm failed: {output:?}");
    assert_eq!(
        common::stdout_of(&output).trim_end_matches('\n'),
        format!("Role: engineer - Active: yes - Source: {input}")
    );
}
