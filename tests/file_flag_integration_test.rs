use std::io::Write;
use std::process::Stdio;
use tempfile::NamedTempFile;

mod common;

use common::command;

/// `-f <file>` reads the named file and does not touch stdin.
/// Reverted (no `-f` support): clap rejects the unknown flag and exits with
/// status 2, so this would fail.
#[test]
fn test_file_flag_reads_file_not_stdin() {
    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, r#"{{"name":"Alice"}}"#).expect("write temp file");

    let output = command()
        .arg("-f")
        .arg(file.path())
        .arg("name")
        .stdin(Stdio::null())
        .output()
        .expect("run parsm");

    assert!(
        output.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Alice");
}

/// Multiple `-f` flags are processed in the order given, each as its own
/// independent document.
#[test]
fn test_multiple_file_flags_preserve_order() {
    let mut file_a = NamedTempFile::new().expect("create temp file a");
    write!(file_a, r#"{{"name":"Alice"}}"#).expect("write temp file a");
    let mut file_b = NamedTempFile::new().expect("create temp file b");
    write!(file_b, r#"{{"name":"Bob"}}"#).expect("write temp file b");

    let output = command()
        .arg("-f")
        .arg(file_a.path())
        .arg("-f")
        .arg(file_b.path())
        .arg("name")
        .stdin(Stdio::null())
        .output()
        .expect("run parsm");

    assert!(
        output.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines, vec!["Alice", "Bob"]);
}

/// `-f -` reads from stdin at that position.
#[test]
fn test_file_flag_dash_reads_stdin() {
    let mut cmd = command();
    cmd.args(["-f", "-", "name"]);
    let result = common::run(cmd, r#"{"name":"Bob"}"#);
    assert!(
        result.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = common::stdout_of(&result);
    assert_eq!(stdout.trim(), "Bob");
}

/// A missing/unreadable file fails fast with exit code 1 and a stderr message
/// naming the path, instead of silently falling back to stdin.
#[test]
fn test_missing_file_exits_with_error() {
    let missing_path = "/no/such/file.json";

    let output = command()
        .arg("-f")
        .arg(missing_path)
        .arg("name")
        .stdin(Stdio::null())
        .output()
        .expect("run parsm");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(missing_path),
        "expected stderr to mention the missing path, got: {stderr}"
    );
}

/// Convert mode (no expression given) still works with `-f`: the file's
/// content is read instead of stdin and written as JSON.
#[test]
fn test_file_flag_convert_mode() {
    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "name: Alice").expect("write temp file");

    let output = command()
        .arg("-f")
        .arg(file.path())
        .stdin(Stdio::null())
        .output()
        .expect("run parsm");

    assert!(
        output.status.success(),
        "parsm failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "{\"name\":\"Alice\"}\n"
    );
}

/// The first positional argument's help surface was relabeled from `[FILTER]`
/// to `[EXPR]`. Reverted, `--help` would show `[FILTER]` instead.
#[test]
fn test_help_shows_expr_not_filter() {
    let output = command().arg("--help").output().expect("run parsm --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[EXPR]"),
        "expected help output to contain [EXPR], got: {stdout}"
    );
    assert!(
        !stdout.contains("[FILTER]"),
        "expected help output to not contain [FILTER], got: {stdout}"
    );
}
