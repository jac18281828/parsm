use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// Helper function to create a Command with proper environment setup
fn parsm_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_parsm"));
    cmd.env("RUST_LOG", "parsm=error");
    cmd
}

/// `-f <file>` reads the named file and does not touch stdin.
/// Reverted (no `-f` support): clap rejects the unknown flag and exits with
/// status 2, so this would fail.
#[test]
fn test_file_flag_reads_file_not_stdin() {
    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, r#"{{"name":"Alice"}}"#).expect("write temp file");

    let output = parsm_command()
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

    let output = parsm_command()
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
    let mut child = parsm_command()
        .arg("-f")
        .arg("-")
        .arg("name")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");

    let mut stdin = child.stdin.take().expect("get stdin");
    write!(stdin, r#"{{"name":"Bob"}}"#).expect("write to stdin");
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

/// A missing/unreadable file fails fast with exit code 1 and a stderr message
/// naming the path, instead of silently falling back to stdin.
#[test]
fn test_missing_file_exits_with_error() {
    let missing_path = "/no/such/file.json";

    let output = parsm_command()
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
/// content flows through `process_stream`. The exact output format of
/// convert mode is out of scope here; only confirm the file's content made
/// it through instead of an empty/stdin read.
#[test]
fn test_file_flag_convert_mode() {
    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "name: Alice").expect("write temp file");

    let output = parsm_command()
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("name"),
        "expected converted output to contain 'name', got: {stdout}"
    );
}

/// The first positional argument's help surface was relabeled from `[FILTER]`
/// to `[EXPR]`. Reverted, `--help` would show `[FILTER]` instead.
#[test]
fn test_help_shows_expr_not_filter() {
    let output = parsm_command()
        .arg("--help")
        .output()
        .expect("run parsm --help");

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
