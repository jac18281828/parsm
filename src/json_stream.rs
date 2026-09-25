//! JSON read as a stream of top-level values, whatever the line layout.
//!
//! Lines are grouped into chunks that end where every value opened in them
//! has closed, so each chunk is parsed once and a record is emitted as soon
//! as its value completes.

use serde_json::{Deserializer, Value};
use std::collections::VecDeque;
use std::io::{self, BufRead};

use crate::detect::Format;
use crate::lines::{Line, LineSource};
use crate::record::Record;
use crate::records::RecordError;

/// Tracks bracket depth and string state across the lines of a chunk.
#[derive(Default)]
struct Nesting {
    depth: usize,
    in_string: bool,
    escaped: bool,
    unbalanced: bool,
}

impl Nesting {
    fn feed(&mut self, line: &str) {
        for byte in line.bytes() {
            if self.in_string {
                match (self.escaped, byte) {
                    (true, _) => self.escaped = false,
                    (false, b'\\') => self.escaped = true,
                    (false, b'"') => self.in_string = false,
                    _ => {}
                }
                continue;
            }
            match byte {
                b'"' => self.in_string = true,
                b'{' | b'[' => self.depth += 1,
                b'}' | b']' => match self.depth.checked_sub(1) {
                    Some(depth) => self.depth = depth,
                    None => self.unbalanced = true,
                },
                _ => {}
            }
        }
    }

    /// A chunk ends at a line end with nothing left open. A string still open
    /// there can never close, since JSON strings hold no raw newline.
    fn chunk_complete(&self) -> bool {
        self.depth == 0 || self.in_string || self.unbalanced
    }
}

/// The lines, starting with `first`, that hold the next complete value.
pub(crate) fn read_chunk<R: BufRead>(
    first: Line,
    source: &mut LineSource<R>,
) -> io::Result<Vec<Line>> {
    let mut nesting = Nesting::default();
    nesting.feed(&first.text);
    let mut chunk = vec![first];
    while !nesting.chunk_complete() {
        let Some(line) = source.next_line()? else {
            break;
        };
        nesting.feed(&line.text);
        chunk.push(line);
    }
    Ok(chunk)
}

/// Whether `chunk` opens JSON input: its first value parses, and only
/// whitespace or further `{…}`/`[…]` values follow it on its last line.
pub(crate) fn opens_json_input(chunk: &[Line]) -> bool {
    let text = chunk_text(chunk);
    let mut values = Deserializer::from_str(&text).into_iter::<Value>();
    if !matches!(values.next(), Some(Ok(_))) {
        return false;
    }
    values.all(|value| matches!(value, Ok(Value::Object(_) | Value::Array(_))))
}

fn chunk_text(chunk: &[Line]) -> String {
    let mut text = String::new();
    for line in chunk {
        text.push_str(&line.text);
        text.push('\n');
    }
    text
}

/// A value that failed to parse, and where reading resumes.
struct Failure {
    line: usize,
    reason: String,
    /// Index of the first chunk line to read again.
    resume_from: usize,
}

struct ParsedChunk {
    values: Vec<(Value, String)>,
    failure: Option<Failure>,
}

fn parse_chunk(chunk: &[Line]) -> ParsedChunk {
    let text = chunk_text(chunk);
    let mut stream = Deserializer::from_str(&text).into_iter::<Value>();
    let mut values = Vec::new();
    let mut start = 0;
    loop {
        match stream.next() {
            None => {
                return ParsedChunk {
                    values,
                    failure: None,
                };
            }
            Some(Ok(value)) => {
                let end = stream.byte_offset();
                values.push((value, text[start..end].trim().to_string()));
                start = end;
            }
            Some(Err(error)) => {
                let failure = locate_failure(&error, chunk, &text, start);
                return ParsedChunk {
                    values,
                    failure: Some(failure),
                };
            }
        }
    }
}

/// Resume at the line after the failure. When the failure falls on the first
/// token of a later line, the failed value ended before that line, so it is
/// read again.
fn locate_failure(error: &serde_json::Error, chunk: &[Line], text: &str, start: usize) -> Failure {
    let reason = reason_of(error);
    let rest = &text[start..];
    let value_start = line_index(text, start + rest.len() - rest.trim_start().len());
    if error.is_eof() {
        return Failure {
            line: chunk[value_start].number,
            reason,
            resume_from: chunk.len(),
        };
    }
    let error_index = error.line().saturating_sub(1).min(chunk.len() - 1);
    let failed_line = &chunk[error_index].text;
    let first_token_column = failed_line.len() - failed_line.trim_start().len() + 1;
    if error_index > value_start && error.column() <= first_token_column {
        Failure {
            line: chunk[value_start].number,
            reason,
            resume_from: error_index,
        }
    } else {
        Failure {
            line: chunk[error_index].number,
            reason,
            resume_from: error_index + 1,
        }
    }
}

/// Index of the chunk line holding byte `offset` of the chunk text.
fn line_index(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count()
}

/// The error message without serde_json's chunk-relative position.
fn reason_of(error: &serde_json::Error) -> String {
    let message = error.to_string();
    let position = format!(" at line {} column {}", error.line(), error.column());
    message
        .strip_suffix(&position)
        .unwrap_or(&message)
        .to_string()
}

/// Records from JSON input: one per top-level value, one per item of a
/// top-level array.
pub(crate) struct JsonRecords<R> {
    source: LineSource<R>,
    ready: VecDeque<Result<Record, RecordError>>,
    started: bool,
    parsed_any: bool,
}

impl<R: BufRead> JsonRecords<R> {
    pub(crate) fn new(source: LineSource<R>) -> Self {
        Self {
            source,
            ready: VecDeque::new(),
            started: false,
            parsed_any: false,
        }
    }

    /// The next chunk; blank lines and leading `#` lines are skipped.
    fn next_chunk(&mut self) -> io::Result<Option<Vec<Line>>> {
        while let Some(line) = self.source.next_line()? {
            if line.is_blank() || (!self.started && line.is_comment()) {
                continue;
            }
            self.started = true;
            return read_chunk(line, &mut self.source).map(Some);
        }
        Ok(None)
    }

    fn queue(&mut self, mut chunk: Vec<Line>) {
        let parsed = parse_chunk(&chunk);
        for (value, source) in parsed.values {
            self.parsed_any = true;
            self.queue_value(value, source);
        }
        if let Some(failure) = parsed.failure {
            self.ready.push_back(Err(RecordError::parse(
                self.parsed_any,
                Format::Json,
                failure.line,
                failure.reason,
            )));
            self.source.replay(chunk.split_off(failure.resume_from));
        }
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
            match self.next_chunk() {
                Ok(Some(chunk)) => self.queue(chunk),
                Ok(None) => return None,
                Err(error) => return Some(Err(RecordError::Io(error))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn source(input: &str) -> LineSource<Cursor<Vec<u8>>> {
        LineSource::new(Cursor::new(input.as_bytes().to_vec()))
    }

    fn first_chunk(input: &str) -> Vec<Line> {
        let mut source = source(input);
        let first = source.next_line().unwrap().unwrap();
        read_chunk(first, &mut source).unwrap()
    }

    fn outcomes(input: &str) -> Vec<Result<String, String>> {
        JsonRecords::new(source(input))
            .map(|item| match item {
                Ok(record) => Ok(record.source().to_string()),
                Err(error) => Err(error.to_string()),
            })
            .collect()
    }

    #[test]
    fn chunk_spans_a_pretty_value() {
        assert_eq!(first_chunk("{\n \"a\": 1\n}\n{\"b\":2}").len(), 3);
        assert_eq!(first_chunk("{\"a\":1}\n{\"b\":2}").len(), 1);
    }

    #[test]
    fn chunk_ignores_brackets_inside_strings() {
        assert_eq!(first_chunk("{\"a\": \"{[\"}\nx").len(), 1);
    }

    #[test]
    fn chunk_ends_at_an_unterminated_string() {
        assert_eq!(first_chunk("{\"text\": \"Hello\nWorld\"}").len(), 1);
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
            assert!(opens_json_input(&first_chunk(input)), "{input}");
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
            assert!(!opens_json_input(&first_chunk(input)), "{input}");
        }
    }

    #[test]
    fn array_items_become_records_with_compact_source() {
        let sources = outcomes("[{\"b\": 1, \"a\": 2}, 3]");
        assert_eq!(
            sources,
            vec![Ok("{\"b\":1,\"a\":2}".to_string()), Ok("3".to_string())]
        );
    }

    #[test]
    fn value_source_is_its_own_text() {
        let sources = outcomes("{\"a\":1} {\"a\":2}\n{\n \"a\": 3\n}");
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
        let results = outcomes("{\"a\":1}\n{\"a\":2\n{\"a\":3}");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Ok("{\"a\":1}".to_string()));
        let warning = results[1].clone().unwrap_err();
        assert!(warning.starts_with("failed to parse line 2: "), "{warning}");
        assert_eq!(results[2], Ok("{\"a\":3}".to_string()));
    }

    #[test]
    fn failure_inside_a_value_resumes_after_its_line() {
        let results = outcomes("{\"a\":1}\n{\"a\": oops}\n{\"a\":3}");
        assert_eq!(results.len(), 3);
        let warning = results[1].clone().unwrap_err();
        assert!(warning.starts_with("failed to parse line 2: "), "{warning}");
        assert!(!warning.contains("column"), "{warning}");
        assert_eq!(results[2], Ok("{\"a\":3}".to_string()));
    }

    #[test]
    fn failure_before_any_value_is_a_first_record_failure() {
        let results = outcomes("hello\n{\"a\":1}");
        assert!(matches!(
            JsonRecords::new(source("hello")).next(),
            Some(Err(RecordError::First { line: 1, .. }))
        ));
        assert_eq!(results[1], Ok("{\"a\":1}".to_string()));
    }

    #[test]
    fn leading_comments_are_skipped() {
        assert_eq!(outcomes("# note\n\n42"), vec![Ok("42".to_string())]);
    }
}
