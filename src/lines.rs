//! Numbered input lines with replay, shared by detection and every format
//! reader.

use std::collections::VecDeque;
use std::io::{self, BufRead};

/// One input line without its terminator, numbered from 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    pub(crate) number: usize,
    pub(crate) text: String,
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
}

impl<R: BufRead> LineSource<R> {
    pub(crate) fn new(reader: R) -> Self {
        Self {
            reader,
            read: 0,
            replay: VecDeque::new(),
        }
    }

    /// The next line, or `None` at end of input.
    pub(crate) fn next_line(&mut self) -> io::Result<Option<Line>> {
        if let Some(line) = self.replay.pop_front() {
            return Ok(Some(line));
        }
        let mut text = String::new();
        if self.reader.read_line(&mut text)? == 0 {
            return Ok(None);
        }
        strip_terminator(&mut text);
        self.read += 1;
        Ok(Some(Line {
            number: self.read,
            text,
        }))
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

fn strip_terminator(text: &mut String) {
    if text.ends_with('\n') {
        text.pop();
        if text.ends_with('\r') {
            text.pop();
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
}
