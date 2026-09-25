//! Helpers shared by the per-format CLI integration tests.
//!
//! Each test crate compiles this module on its own and uses a subset of it.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// How long a streamed record may take to reach stdout while stdin is open.
const STREAM_TIMEOUT: Duration = Duration::from_secs(5);

/// Run `cmd` with `stdin` as its whole input and collect its output.
pub fn run(mut cmd: Command, stdin: &str) -> Output {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");
    let mut input = child.stdin.take().expect("child stdin");
    input.write_all(stdin.as_bytes()).expect("write stdin");
    drop(input);
    child.wait_with_output().expect("wait for parsm")
}

/// Output of a run whose first line was read while stdin was still open.
pub struct Streamed {
    /// The first stdout line, if it arrived before stdin closed.
    pub first: Option<String>,
    /// Everything stdout produced after the first line.
    pub rest: String,
    pub status: ExitStatus,
}

/// Write `first` as a line, wait for one stdout line while stdin stays open,
/// then write `second` and close stdin.
pub fn first_line_while_open(mut cmd: Command, first: &str, second: &str) -> Streamed {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn parsm");
    let mut input = child.stdin.take().expect("child stdin");
    let output = child.stdout.take().expect("child stdout");

    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut output = BufReader::new(output);
        let mut line = String::new();
        output.read_line(&mut line).expect("read first line");
        // The receiver is gone once the first line has timed out.
        let _ = sender.send(line);
        let mut rest = String::new();
        output.read_to_string(&mut rest).expect("read rest");
        rest
    });

    writeln!(input, "{first}").expect("write first line");
    input.flush().expect("flush first line");
    let first = receiver
        .recv_timeout(STREAM_TIMEOUT)
        .ok()
        .map(|line| line.trim_end_matches('\n').to_string());

    writeln!(input, "{second}").expect("write second line");
    drop(input);
    let rest = reader.join().expect("reader thread");
    let status = child.wait().expect("wait for parsm");
    Streamed {
        first,
        rest,
        status,
    }
}
