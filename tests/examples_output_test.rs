use std::process::Stdio;

mod common;

use common::command;

/// `--examples` exits 0 and prints every section heading, so a rewrite of
/// `print_usage_examples` cannot silently drop a section.
#[test]
fn examples_flag_prints_every_section() {
    let output = command()
        .arg("--examples")
        .stdin(Stdio::null())
        .output()
        .expect("run parsm --examples");

    assert!(
        output.status.success(),
        "parsm --examples failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for heading in [
        "EXAMPLES:",
        "OPERATORS:",
        "FIELD ACCESS:",
        "TEMPLATE FORMATS:",
        "TEMPLATE VARIABLES:",
        "FORMAT FLAGS:",
    ] {
        assert!(
            stdout.contains(heading),
            "expected --examples output to contain {heading:?}, got: {stdout}"
        );
    }
}
