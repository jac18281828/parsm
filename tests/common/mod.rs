//! Helpers shared by the per-format CLI integration tests.
//!
//! Each test crate compiles this module on its own and uses a subset of it.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// How long streamed records may take to reach stdout while stdin is open.
const STREAM_TIMEOUT: Duration = Duration::from_secs(5);

/// Run `cmd` with `stdin` as its whole input and collect its output.
pub fn run(mut cmd: Command, stdin: impl AsRef<[u8]>) -> Output {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");
    let mut input = child.stdin.take().expect("child stdin");
    input.write_all(stdin.as_ref()).expect("write stdin");
    drop(input);
    child.wait_with_output().expect("wait for parsm")
}

/// A run's stdout as text.
pub fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A running parsm whose stdin stays open until [`Session::finish`].
pub struct Session {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
    reader: JoinHandle<()>,
}

impl Session {
    pub fn start(mut cmd: Command) -> Self {
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn parsm");
        let input = child.stdin.take().expect("child stdin");
        let output = child.stdout.take().expect("child stdout");
        let (sender, lines) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let line = line.expect("read stdout line");
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            lines,
            reader,
        }
    }

    pub fn write_line(&mut self, line: &str) {
        writeln!(self.input, "{line}").expect("write stdin line");
        self.input.flush().expect("flush stdin");
    }

    /// Up to `count` stdout lines that arrive within the stream timeout.
    pub fn read_lines(&mut self, count: usize) -> Vec<String> {
        let deadline = Instant::now() + STREAM_TIMEOUT;
        let mut lines = Vec::new();
        while lines.len() < count {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) => lines.push(line),
                Err(_) => break,
            }
        }
        lines
    }

    /// Close stdin; the stdout lines not yet read and the exit status.
    pub fn finish(self) -> (Vec<String>, ExitStatus) {
        let Session {
            mut child,
            input,
            lines,
            reader,
        } = self;
        drop(input);
        reader.join().expect("reader thread");
        let status = child.wait().expect("wait for parsm");
        (lines.try_iter().collect(), status)
    }
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
pub fn first_line_while_open(cmd: Command, first: &str, second: &str) -> Streamed {
    let mut session = Session::start(cmd);
    session.write_line(first);
    let first = session.read_lines(1).pop();
    session.write_line(second);
    let (rest, status) = session.finish();
    Streamed {
        first,
        rest: rest.iter().map(|line| format!("{line}\n")).collect(),
        status,
    }
}
