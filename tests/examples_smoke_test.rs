use std::process::Command;

fn run_example(name: &str) -> String {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", name])
        .output()
        .expect("failed to invoke cargo run --example");
    assert!(
        output.status.success(),
        "example {name} exited with {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("example stdout should be valid utf8")
}

#[test]
fn filter_and_template_example_produces_expected_matches() {
    let stdout = run_example("filter_and_template");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec!["Alice (30) - admin", "Dave (35) - moderator"],
        "filter_and_template example output changed: {stdout}"
    );
}

#[test]
fn streaming_format_detection_example_detects_each_format() {
    let stdout = run_example("streaming_format_detection");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec![
            r#"json: json {"name":"Alice","age":30}"#,
            r#"csv: csv ["Alice","30","Engineer"]"#,
            r#"logfmt: logfmt {"level":"error","msg":"timeout","service":"api"}"#,
        ],
        "streaming_format_detection example output changed: {stdout}"
    );
}
