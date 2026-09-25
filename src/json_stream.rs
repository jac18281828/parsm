//! JSON read as a stream of top-level values, whatever the line layout.
//!
//! serde_json reads the input through a line feed: a value is emitted as soon
//! as it completes, and a syntax error surfaces on the line that holds it,
//! never later. Every fed line is kept until its values are emitted, so each
//! value's source text and each failure's line are known.

use serde_json::de::IoRead;
use serde_json::{Deserializer, StreamDeserializer, Value};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Read};
use std::rc::Rc;

use crate::detect::Format;
use crate::lines::{Line, LineSource, UNDECODABLE};
use crate::record::Record;
use crate::records::RecordError;

/// Lines handed to serde_json since its stream began.
#[derive(Default)]
struct Fed {
    /// Each kept line with the stream offset where it starts.
    lines: VecDeque<(usize, Line)>,
    /// Stream text from offset `base` on.
    text: String,
    base: usize,
    /// Lines dropped from the front since the stream began.
    dropped: usize,
    /// An undecoded line the feed refused.
    undecodable: Option<usize>,
    /// A read failure the feed passed to serde_json.
    io_error: Option<io::Error>,
}

impl Fed {
    fn push(&mut self, line: &Line) {
        let offset = self.base + self.text.len();
        self.text.push_str(&line.text);
        self.text.push('\n');
        self.lines.push_back((offset, line.clone()));
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        &self.text[start - self.base..end - self.base]
    }

    /// Index into `lines` of the line holding stream offset `offset`.
    fn index_at(&self, offset: usize) -> usize {
        self.lines
            .partition_point(|&(start, _)| start <= offset)
            .saturating_sub(1)
    }

    fn number_at(&self, index: usize) -> usize {
        self.lines.get(index).map_or(0, |(_, line)| line.number)
    }

    /// Drop lines that end at or before `offset`.
    fn forget_before(&mut self, offset: usize) {
        while self.lines.len() > 1 && self.lines[1].0 <= offset {
            self.lines.pop_front();
            self.dropped += 1;
        }
        if let Some(&(start, _)) = self.lines.front() {
            self.text.drain(..start - self.base);
            self.base = start;
        }
    }

    /// Text from `offset` to the end of its line.
    fn rest_of_line(&self, offset: usize) -> &str {
        let rest = &self.text[offset - self.base..];
        rest.split('\n').next().unwrap_or_default()
    }

    /// The opening value ending at `end`, and the lines after it.
    fn into_opening(self, value: Value, end: usize) -> Opening {
        let source = self.slice(self.base, end).trim().to_string();
        let last = self.index_at(end.saturating_sub(1));
        let mut rest = Vec::new();
        if let Some((_, line)) = self.lines.get(last) {
            let remainder = self.rest_of_line(end);
            if !remainder.trim().is_empty() {
                rest.push(Line {
                    number: line.number,
                    text: remainder.to_string(),
                    decoded: true,
                });
            }
        }
        rest.extend(self.into_lines_from(last + 1));
        Opening {
            value,
            source,
            rest,
        }
    }

    fn into_lines_from(self, index: usize) -> Vec<Line> {
        self.lines
            .into_iter()
            .skip(index)
            .map(|(_, line)| line)
            .collect()
    }
}

/// Where the feed's lines come from.
trait Supply {
    fn next_line(&mut self) -> io::Result<Option<Line>>;
}

/// Record reading: the content lines of a shared source.
struct Shared<R>(Rc<RefCell<LineSource<R>>>);

impl<R: BufRead> Supply for Shared<R> {
    fn next_line(&mut self) -> io::Result<Option<Line>> {
        self.0.borrow_mut().next_content_line()
    }
}

/// Detection: the probe line, then every following line.
struct Probe<'a, R> {
    first: Option<Line>,
    source: &'a mut LineSource<R>,
}

impl<R: BufRead> Supply for Probe<'_, R> {
    fn next_line(&mut self) -> io::Result<Option<Line>> {
        match self.first.take() {
            Some(line) => Ok(Some(line)),
            None => self.source.next_line(),
        }
    }
}

/// Serves supplied lines to serde_json, recording each.
struct Feed<S> {
    supply: S,
    fed: Rc<RefCell<Fed>>,
    current: Vec<u8>,
    position: usize,
}

impl<S> Feed<S> {
    fn new(supply: S, fed: Rc<RefCell<Fed>>) -> Self {
        Self {
            supply,
            fed,
            current: Vec::new(),
            position: 0,
        }
    }
}

impl<S: Supply> Read for Feed<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.position == self.current.len() {
            let line = match self.supply.next_line() {
                Ok(Some(line)) => line,
                Ok(None) => return Ok(0),
                Err(error) => {
                    let kind = error.kind();
                    self.fed.borrow_mut().io_error = Some(error);
                    return Err(io::Error::new(kind, "read failed"));
                }
            };
            let mut fed = self.fed.borrow_mut();
            fed.push(&line);
            if !line.decoded {
                fed.undecodable = Some(line.number);
                return Err(io::Error::new(io::ErrorKind::InvalidData, UNDECODABLE));
            }
            self.current.clear();
            self.current.extend_from_slice(line.text.as_bytes());
            self.current.push(b'\n');
            self.position = 0;
        }
        let count = buf.len().min(self.current.len() - self.position);
        buf[..count].copy_from_slice(&self.current[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

/// The feed returns at most one line per read, so the buffer never reads
/// past the line serde_json is parsing.
type Values<S> = StreamDeserializer<'static, IoRead<BufReader<Feed<S>>>, Value>;

/// The first value of input detected as JSON, parsed once, and the lines
/// that follow it: the rest of its last line, then every later line read.
pub(crate) struct Opening {
    value: Value,
    source: String,
    pub(crate) rest: Vec<Line>,
}

/// Whether the input opens with JSON: its first value parses, and only
/// whitespace or further `{…}`/`[…]` values follow it on its last line.
/// When it does not, every line read, the probe included, is appended to
/// `read`.
pub(crate) fn opens_json_input<R: BufRead>(
    probe: Line,
    source: &mut LineSource<R>,
    read: &mut Vec<Line>,
) -> io::Result<Option<Opening>> {
    let fed = Rc::new(RefCell::new(Fed::default()));
    let supply = Probe {
        first: Some(probe),
        source,
    };
    let feed = BufReader::new(Feed::new(supply, fed.clone()));
    let mut values: Values<Probe<'_, R>> = Deserializer::from_reader(feed).into_iter();
    let first = match values.next() {
        Some(Ok(value)) => Some((value, values.byte_offset())),
        _ => None,
    };
    let opens = first
        .as_ref()
        .is_some_and(|&(_, end)| only_containers_follow(values, &fed, end));
    let mut fed = fed.take();
    if let Some(error) = fed.io_error.take() {
        return Err(error);
    }
    match first {
        Some((value, end)) if opens => Ok(Some(fed.into_opening(value, end))),
        _ => {
            read.extend(fed.into_lines_from(0));
            Ok(None)
        }
    }
}

/// Whether only whitespace or `{…}`/`[…]` values follow offset `end` on its
/// line, and on the lines those values end on.
fn only_containers_follow<S: Supply>(
    mut values: Values<S>,
    fed: &RefCell<Fed>,
    mut end: usize,
) -> bool {
    loop {
        let opens_container = {
            let fed = fed.borrow();
            let rest = fed.rest_of_line(end).trim_start();
            if rest.is_empty() {
                return true;
            }
            rest.starts_with(['{', '['])
        };
        if !opens_container || !matches!(values.next(), Some(Ok(_))) {
            return false;
        }
        end = values.byte_offset();
    }
}

/// A value that failed to parse, and where reading resumes.
struct Failure {
    line: usize,
    reason: String,
    /// Index of the first fed line to read again.
    resume_from: usize,
}

/// Resume at the line after the failure. When the failure falls on the first
/// token of a later line, the failed value ended before that line, so it is
/// read again.
fn locate_failure(error: &serde_json::Error, fed: &Fed, start: usize) -> Failure {
    let reason = reason_of(error);
    let rest = &fed.text[start - fed.base..];
    let value_index = fed.index_at(start + rest.len() - rest.trim_start().len());
    if error.is_eof() {
        return Failure {
            line: fed.number_at(value_index),
            reason,
            resume_from: fed.lines.len(),
        };
    }
    let error_index = (error.line().saturating_sub(1))
        .saturating_sub(fed.dropped)
        .min(fed.lines.len().saturating_sub(1));
    let failed_line = fed.lines.get(error_index).map_or("", |(_, l)| &l.text);
    let first_token_column = failed_line.len() - failed_line.trim_start().len() + 1;
    if error_index > value_index && error.column() <= first_token_column {
        Failure {
            line: fed.number_at(value_index),
            reason,
            resume_from: error_index,
        }
    } else {
        Failure {
            line: fed.number_at(error_index),
            reason,
            resume_from: error_index + 1,
        }
    }
}

/// The error message without serde_json's stream-relative position.
fn reason_of(error: &serde_json::Error) -> String {
    let position = format!(" at line {} column {}", error.line(), error.column());
    error.to_string().replace(&position, "")
}

/// Records from JSON input: one per top-level value, one per item of a
/// top-level array.
pub(crate) struct JsonRecords<R: BufRead> {
    source: Rc<RefCell<LineSource<R>>>,
    fed: Rc<RefCell<Fed>>,
    values: Option<Values<Shared<R>>>,
    /// Stream offset where the next value's text begins.
    start: usize,
    ready: VecDeque<Result<Record, RecordError>>,
    parsed_any: bool,
    finished: bool,
}

impl<R: BufRead> JsonRecords<R> {
    /// Records from `source`, beginning with the detected `opening` value
    /// when there is one.
    pub(crate) fn new(mut source: LineSource<R>, opening: Option<Opening>) -> Self {
        if opening.is_some() {
            source.mark_content_started();
        }
        let mut records = Self {
            source: Rc::new(RefCell::new(source)),
            fed: Rc::new(RefCell::new(Fed::default())),
            values: None,
            start: 0,
            ready: VecDeque::new(),
            parsed_any: false,
            finished: false,
        };
        if let Some(opening) = opening {
            records.parsed_any = true;
            records.queue_value(opening.value, opening.source);
        }
        records
    }

    /// Read the next value, queueing its records or its failure.
    fn advance(&mut self) {
        let (source, fed) = (&self.source, &self.fed);
        let values = self.values.get_or_insert_with(|| {
            let feed = BufReader::new(Feed::new(Shared(source.clone()), fed.clone()));
            Deserializer::from_reader(feed).into_iter()
        });
        let item = values.next();
        let end = values.byte_offset();
        match item {
            None => self.finished = true,
            Some(Ok(value)) => {
                let text = self.fed.borrow().slice(self.start, end).trim().to_string();
                self.fed.borrow_mut().forget_before(end);
                self.start = end;
                self.parsed_any = true;
                self.queue_value(value, text);
            }
            Some(Err(error)) => self.fail(&error),
        }
    }

    /// Queue the failure and restart the stream where reading resumes.
    fn fail(&mut self, error: &serde_json::Error) {
        self.values = None;
        let start = std::mem::take(&mut self.start);
        let mut fed = self.fed.take();
        if let Some(io_error) = fed.io_error.take() {
            self.ready.push_back(Err(RecordError::Io(io_error)));
            self.finished = true;
            return;
        }
        let failure = match fed.undecodable {
            Some(line) if error.is_io() => Failure {
                line,
                reason: UNDECODABLE.to_string(),
                resume_from: fed.lines.len(),
            },
            _ => locate_failure(error, &fed, start),
        };
        self.ready.push_back(Err(RecordError::parse(
            self.parsed_any,
            Format::Json,
            failure.line,
            failure.reason,
        )));
        let resume = fed.into_lines_from(failure.resume_from);
        self.source.borrow_mut().replay(resume);
    }

    fn queue_value(&mut self, value: Value, source: String) {
        match value {
            Value::Array(items) => {
                for item in items {
                    let source = item.to_string();
                    self.ready.push_back(Ok(Record::value(item, source)));
                }
            }
            value => self.ready.push_back(Ok(Record::value(value, source))),
        }
    }
}

impl<R: BufRead> Iterator for JsonRecords<R> {
    type Item = Result<Record, RecordError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.ready.pop_front() {
                return Some(item);
            }
            if self.finished {
                return None;
            }
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn source(input: &[u8]) -> LineSource<Cursor<Vec<u8>>> {
        LineSource::new(Cursor::new(input.to_vec()))
    }

    fn opens(input: &str) -> bool {
        let mut source = source(input.as_bytes());
        let probe = source.next_line().unwrap().unwrap();
        opens_json_input(probe, &mut source, &mut Vec::new())
            .unwrap()
            .is_some()
    }

    fn outcomes(input: &[u8]) -> Vec<Result<String, String>> {
        JsonRecords::new(source(input), None)
            .map(|item| match item {
                Ok(record) => Ok(record.source().to_string()),
                Err(error) => Err(error.to_string()),
            })
            .collect()
    }

    #[test]
    fn trailing_rule_accepts_values_and_objects() {
        for input in [
            "42",
            "true",
            "\"hi\"",
            "[1,2]",
            "{\"a\":1} {\"a\":2}",
            "{\n}",
        ] {
            assert!(opens(input), "{input}");
        }
    }

    #[test]
    fn trailing_rule_rejects_scalars_and_text_after_the_first_value() {
        for input in [
            "1,2,3",
            "\"Smith, John\",30",
            "200 OK",
            "1 2 3",
            "hello",
            "[a]",
        ] {
            assert!(!opens(input), "{input}");
        }
    }

    #[test]
    fn detection_stops_at_the_first_error() {
        let mut source = source(b"2024 ERROR [worker {id=3\nnext\nlast\n");
        let probe = source.next_line().unwrap().unwrap();
        let mut read = Vec::new();
        assert!(
            opens_json_input(probe, &mut source, &mut read)
                .unwrap()
                .is_none()
        );
        assert_eq!(read.len(), 1);
        assert_eq!(source.next_line().unwrap().unwrap().text, "next");
    }

    #[test]
    fn opening_hands_over_the_first_value_and_the_lines_after_it() {
        let mut source = source(b"{\n\"a\": 1\n} {\"b\": 2}\nnext\n");
        let probe = source.next_line().unwrap().unwrap();
        let mut read = Vec::new();
        let opening = opens_json_input(probe, &mut source, &mut read)
            .unwrap()
            .unwrap();
        assert!(read.is_empty());
        assert_eq!(opening.source, "{\n\"a\": 1\n}");
        let rest: Vec<(usize, &str)> = opening
            .rest
            .iter()
            .map(|l| (l.number, l.text.as_str()))
            .collect();
        assert_eq!(rest, vec![(3, " {\"b\": 2}")]);
        assert_eq!(source.next_line().unwrap().unwrap().text, "next");
    }

    #[test]
    fn failed_detection_replays_every_line_read() {
        let mut source = source(b"{\n\"a\": x\n}\n");
        let probe = source.next_line().unwrap().unwrap();
        let mut read = Vec::new();
        assert!(
            opens_json_input(probe, &mut source, &mut read)
                .unwrap()
                .is_none()
        );
        let texts: Vec<&str> = read.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["{", "\"a\": x"]);
    }

    #[test]
    fn array_items_become_records_with_compact_source() {
        let sources = outcomes(b"[{\"b\": 1, \"a\": 2}, 3]");
        assert_eq!(
            sources,
            vec![Ok("{\"b\":1,\"a\":2}".to_string()), Ok("3".to_string())]
        );
    }

    #[test]
    fn value_source_is_its_own_text() {
        let sources = outcomes(b"{\"a\":1} {\"a\":2}\n{\n \"a\": 3\n}");
        assert_eq!(
            sources,
            vec![
                Ok("{\"a\":1}".to_string()),
                Ok("{\"a\":2}".to_string()),
                Ok("{\n \"a\": 3\n}".to_string()),
            ]
        );
    }

    #[test]
    fn truncated_line_fails_and_the_next_line_is_read_again() {
        let results = outcomes(b"{\"a\":1}\n{\"a\":2\n{\"a\":3}");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Ok("{\"a\":1}".to_string()));
        let warning = results[1].clone().unwrap_err();
        assert!(warning.starts_with("failed to parse line 2: "), "{warning}");
        assert!(!warning.contains("column"), "{warning}");
        assert_eq!(results[2], Ok("{\"a\":3}".to_string()));
    }

    #[test]
    fn failure_inside_a_value_resumes_after_its_line() {
        let results = outcomes(b"{\"a\":1}\n{\"a\": oops}\n{\"a\":3}");
        assert_eq!(results.len(), 3);
        let warning = results[1].clone().unwrap_err();
        assert!(warning.starts_with("failed to parse line 2: "), "{warning}");
        assert_eq!(results[2], Ok("{\"a\":3}".to_string()));
    }

    #[test]
    fn failure_lines_count_from_the_input_start() {
        let results = outcomes(b"{\"a\":1}\n\n{\"a\":2}\n{\"a\": x}\n{\"a\":3}");
        let warning = results[2].clone().unwrap_err();
        assert!(warning.starts_with("failed to parse line 4: "), "{warning}");
        assert_eq!(results[3], Ok("{\"a\":3}".to_string()));
    }

    #[test]
    fn undecodable_line_is_skipped_after_the_first_record() {
        let results = outcomes(b"{\"a\":1}\n\xff\n{\"a\":3}");
        assert_eq!(
            results,
            vec![
                Ok("{\"a\":1}".to_string()),
                Err("failed to parse line 2: invalid UTF-8".to_string()),
                Ok("{\"a\":3}".to_string()),
            ]
        );
    }

    #[test]
    fn failure_before_any_value_is_a_first_record_failure() {
        assert!(matches!(
            JsonRecords::new(source(b"hello"), None).next(),
            Some(Err(RecordError::First { line: 1, .. }))
        ));
        assert!(matches!(
            JsonRecords::new(source(b"\xff\n{}"), None).next(),
            Some(Err(RecordError::First { line: 1, .. }))
        ));
    }

    #[test]
    fn leading_comments_are_skipped() {
        assert_eq!(outcomes(b"# note\n\n42"), vec![Ok("42".to_string())]);
    }
}
