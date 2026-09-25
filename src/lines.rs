//! Numbered input lines with replay, shared by detection and every format
//! reader.

use std::collections::VecDeque;
use std::io::{self, BufRead};

/// Reason reported for a line that is not valid UTF-8.
pub(crate) const UNDECODABLE: &str = "invalid UTF-8";

/// One input line without its terminator, numbered from 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    pub(crate) number: usize,
    /// The line's text; lossily decoded when `decoded` is false.
    pub(crate) text: String,
    /// Whether the line was valid UTF-8. Readers report an undecoded line as
    /// a failed record rather than parse its lossy text.
    pub(crate) decoded: bool,
}

impl Line {
    pub(crate) fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    pub(crate) fn is_comment(&self) -> bool {
        self.text.trim_start().starts_with('#')
    }
}

/// Reads numbered lines, serving replayed lines before fresh input.
pub(crate) struct LineSource<R> {
    reader: R,
    read: usize,
    replay: VecDeque<Line>,
    buffer: Vec<u8>,
    content_started: bool,
}

impl<R: BufRead> LineSource<R> {
    pub(crate) fn new(reader: R) -> Self {
        Self {
            reader,
            read: 0,
            replay: VecDeque::new(),
            buffer: Vec::new(),
            content_started: false,
        }
    }

    /// The next line, or `None` at end of input.
    pub(crate) fn next_line(&mut self) -> io::Result<Option<Line>> {
        if let Some(line) = self.replay.pop_front() {
            return Ok(Some(line));
        }
        self.buffer.clear();
        if self.reader.read_until(b'\n', &mut self.buffer)? == 0 {
            return Ok(None);
        }
        strip_terminator(&mut self.buffer);
        self.read += 1;
        let (text, decoded) = match std::str::from_utf8(&self.buffer) {
            Ok(text) => (text.to_string(), true),
            Err(_) => (String::from_utf8_lossy(&self.buffer).into_owned(), false),
        };
        Ok(Some(Line {
            number: self.read,
            text,
            decoded,
        }))
    }

    /// The next line that holds content: blank lines are skipped, and so are
    /// `#` lines until the first content line.
    pub(crate) fn next_content_line(&mut self) -> io::Result<Option<Line>> {
        while let Some(line) = self.next_line()? {
            if line.is_blank() || (!self.content_started && line.is_comment()) {
                continue;
            }
            self.content_started = true;
            return Ok(Some(line));
        }
        Ok(None)
    }

    /// Treat later `#` lines as content, as after a first content line.
    pub(crate) fn mark_content_started(&mut self) {
        self.content_started = true;
    }

    /// Serve `lines`, in order, before anything not yet replayed.
    pub(crate) fn replay(&mut self, lines: Vec<Line>) {
        for line in lines.into_iter().rev() {
            self.replay.push_front(line);
        }
    }

    /// Every line left in the input, replayed lines first.
    pub(crate) fn read_to_end(&mut self) -> io::Result<Vec<Line>> {
        let mut lines = Vec::new();
        while let Some(line) = self.next_line()? {
            lines.push(line);
        }
        Ok(lines)
    }
}

fn strip_terminator(bytes: &mut Vec<u8>) {
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn texts(lines: &[Line]) -> Vec<(usize, &str)> {
        lines.iter().map(|l| (l.number, l.text.as_str())).collect()
    }

    #[test]
    fn numbers_lines_and_strips_terminators() {
        let mut source = LineSource::new(Cursor::new("a\r\nb\n\nc"));
        let lines = source.read_to_end().unwrap();
        assert_eq!(texts(&lines), vec![(1, "a"), (2, "b"), (3, ""), (4, "c")]);
    }

    #[test]
    fn replayed_lines_come_first_in_order() {
        let mut source = LineSource::new(Cursor::new("a\nb\nc\n"));
        let first = source.next_line().unwrap().unwrap();
        let second = source.next_line().unwrap().unwrap();
        source.replay(vec![first, second]);
        let lines = source.read_to_end().unwrap();
        assert_eq!(texts(&lines), vec![(1, "a"), (2, "b"), (3, "c")]);
    }

    #[test]
    fn invalid_utf8_line_is_marked_not_decoded() {
        let mut source = LineSource::new(Cursor::new(b"ok\n\xff\xfe\nok\n".to_vec()));
        let lines = source.read_to_end().unwrap();
        let decoded: Vec<bool> = lines.iter().map(|l| l.decoded).collect();
        assert_eq!(decoded, vec![true, false, true]);
        assert_eq!(lines[1].number, 2);
    }

    #[test]
    fn content_lines_skip_blanks_and_leading_comments() {
        let mut source = LineSource::new(Cursor::new("# a\n\nx\n# b\n\ny\n"));
        let mut lines = Vec::new();
        while let Some(line) = source.next_content_line().unwrap() {
            lines.push(line);
        }
        assert_eq!(texts(&lines), vec![(3, "x"), (4, "# b"), (6, "y")]);
    }
}
