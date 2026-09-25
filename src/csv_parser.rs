//! CSV rows as records, keyed by header names when the input has a header
//! row.
//!
//! One CSV reader parses the whole input through a line feed, so a row is
//! emitted as soon as its line ends and a quoted field may span lines.

use std::collections::VecDeque;
use std::io::{self, BufRead, Read};
use std::sync::Arc;

use crate::detect::Format;
use crate::lines::{Line, LineSource, UNDECODABLE};
use crate::parse::parse_csv_line;
use crate::record::{CsvHeader, Record};
use crate::records::RecordError;

/// Data rows sampled after the first row to decide whether it is a header.
const HEADER_SAMPLE_ROWS: usize = 5;

/// Serves content lines to the CSV reader one at a time, recording each;
/// undecoded lines are held back for reporting.
struct RowFeed<R> {
    source: LineSource<R>,
    current: Vec<u8>,
    position: usize,
    fed: Vec<Line>,
    undecodable: Vec<usize>,
    io_error: Option<io::Error>,
}

impl<R: BufRead> RowFeed<R> {
    /// The next decoded content line, recording undecoded ones.
    fn next_decoded(&mut self) -> io::Result<Option<Line>> {
        while let Some(line) = self.source.next_content_line()? {
            if line.decoded {
                return Ok(Some(line));
            }
            self.undecodable.push(line.number);
        }
        Ok(None)
    }
}

impl<R: BufRead> Read for RowFeed<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.position == self.current.len() {
            let line = match self.next_decoded() {
                Ok(Some(line)) => line,
                Ok(None) => return Ok(0),
                Err(error) => {
                    let kind = error.kind();
                    self.io_error = Some(error);
                    return Err(io::Error::new(kind, "read failed"));
                }
            };
            self.current.clear();
            self.current.extend_from_slice(line.text.as_bytes());
            self.current.push(b'\n');
            self.position = 0;
            self.fed.push(line);
        }
        let count = buf.len().min(self.current.len() - self.position);
        buf[..count].copy_from_slice(&self.current[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

/// What reading the CSV input produced, in input order.
enum Event {
    Row {
        fields: csv::StringRecord,
        source: String,
    },
    /// The row chosen as the header.
    Header(Arc<CsvHeader>),
    Failed {
        line: usize,
        reason: String,
    },
    Io(io::Error),
}

/// Records from CSV input. The first row and up to [`HEADER_SAMPLE_ROWS`]
/// data rows are buffered for header detection; later rows stream.
pub(crate) struct CsvRecords<R: BufRead> {
    reader: csv::Reader<RowFeed<R>>,
    events: VecDeque<Event>,
    header: Option<Arc<CsvHeader>>,
    primed: bool,
    finished: bool,
    parsed_any: bool,
}

impl<R: BufRead> CsvRecords<R> {
    pub(crate) fn new(source: LineSource<R>) -> Self {
        let feed = RowFeed {
            source,
            current: Vec::new(),
            position: 0,
            fed: Vec::new(),
            undecodable: Vec::new(),
            io_error: None,
        };
        let reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(feed);
        Self {
            reader,
            events: VecDeque::new(),
            header: None,
            primed: false,
            finished: false,
            parsed_any: false,
        }
    }

    /// Read one CSV row, queueing it after any lines skipped before it.
    fn read_event(&mut self) {
        let mut fields = csv::StringRecord::new();
        let result = self.reader.read_record(&mut fields);
        let feed = self.reader.get_mut();
        let lines = std::mem::take(&mut feed.fed);
        for line in feed.undecodable.drain(..) {
            self.events.push_back(Event::Failed {
                line,
                reason: UNDECODABLE.to_string(),
            });
        }
        if let Some(error) = feed.io_error.take() {
            self.events.push_back(Event::Io(error));
            self.finished = true;
            return;
        }
        match result {
            Ok(true) => {
                let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
                self.events.push_back(Event::Row {
                    fields,
                    source: texts.join("\n"),
                });
            }
            Ok(false) => self.finished = true,
            Err(error) => self.events.push_back(Event::Failed {
                line: lines.first().map_or(0, |line| line.number),
                reason: error.to_string(),
            }),
        }
    }

    /// Buffer the first rows and turn the first into the header when the
    /// sample says it is one.
    fn prime(&mut self) {
        self.primed = true;
        let rows = |events: &VecDeque<Event>| {
            events
                .iter()
                .filter(|event| matches!(event, Event::Row { .. }))
                .count()
        };
        while !self.finished && rows(&self.events) <= HEADER_SAMPLE_ROWS {
            self.read_event();
        }
        let sample: Vec<&str> = self
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Row { source, .. } => Some(source.as_str()),
                _ => None,
            })
            .collect();
        if !detect_header_row(&sample) {
            return;
        }
        let first_row = self
            .events
            .iter()
            .position(|event| matches!(event, Event::Row { .. }));
        if let Some(index) = first_row
            && let Some(Event::Row { fields, .. }) = self.events.get(index)
        {
            self.events[index] = Event::Header(Arc::new(CsvHeader::new(fields)));
        }
    }
}

impl<R: BufRead> Iterator for CsvRecords<R> {
    type Item = Result<Record, RecordError>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.primed {
            self.prime();
        }
        loop {
            let Some(event) = self.events.pop_front() else {
                if self.finished {
                    return None;
                }
                self.read_event();
                continue;
            };
            return Some(match event {
                Event::Header(header) => {
                    self.header = Some(header);
                    self.parsed_any = true;
                    continue;
                }
                Event::Row { fields, source } => {
                    self.parsed_any = true;
                    Ok(Record::row(&fields, self.header.clone(), source))
                }
                Event::Failed { line, reason } => Err(RecordError::parse(
                    self.parsed_any,
                    Format::Csv,
                    line,
                    reason,
                )),
                Event::Io(error) => Err(RecordError::Io(error)),
            });
        }
    }
}

/// Detects a header row in CSV data by analyzing the first row and sample data rows.
fn detect_header_row(lines: &[&str]) -> bool {
    if lines.len() < 2 {
        return false;
    }

    let first_row = lines[0];
    if let Some(record) = parse_csv_line(first_row) {
        if record.iter().any(|field| is_numeric(field.trim())) {
            return false;
        }

        if record.iter().any(|field| field.trim().is_empty()) {
            return false;
        }

        let sample_size = std::cmp::min(lines.len() - 1, 5);
        for line in lines.iter().take(sample_size + 1).skip(1) {
            if let Some(data_record) = parse_csv_line(line)
                && data_record.iter().any(|field| is_data_like(field.trim()))
            {
                return true;
            }
        }

        let first_row_has_header_names = record.iter().any(|field| {
            let field = field.trim();
            field.contains('_')
                || field.contains(' ')
                || field
                    .chars()
                    .all(|c| c.is_alphabetic() && !c.is_uppercase())
        });

        return first_row_has_header_names;
    }

    false
}

/// Returns true if the field has data-like characteristics (numeric, emails, URLs, or hyphens).
fn is_data_like(field: &str) -> bool {
    is_numeric(field)
        || field.contains('@')
        || field.contains("http")
        || (field.contains('-') && !field.contains('_'))
}

/// Returns true if the field contains only numeric data (with optional signs, dots, or whitespace).
fn is_numeric(field: &str) -> bool {
    !field.is_empty()
        && field.chars().any(|c| c.is_ascii_digit())
        && field
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_detection_with_names() {
        let input = "name,age,occupation\nTom,45,engineer\nAlice,30,doctor";
        let lines: Vec<&str> = input.lines().collect();
        assert!(detect_header_row(&lines));
    }

    #[test]
    fn test_header_detection_no_headers() {
        let input = "Tom,45,engineer\nAlice,30,doctor";
        let lines: Vec<&str> = input.lines().collect();
        assert!(!detect_header_row(&lines));
    }

    #[test]
    fn test_no_header_with_mixed_types() {
        let input = "Alice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager";
        let lines: Vec<&str> = input.lines().collect();
        assert!(!detect_header_row(&lines));
    }

    #[test]
    fn test_header_detection_all_text_headers() {
        let input = "first_name,last_name,job_title\nAlice,Smith,Engineer\nBob,Jones,Designer";
        let lines: Vec<&str> = input.lines().collect();
        assert!(detect_header_row(&lines));
    }

    #[test]
    fn test_header_detection_with_special_chars() {
        let input = "user_id,email_address,signup_date\njohn123,john@example.com,2023-01-15\nmary456,mary@example.com,2023-02-20";
        let lines: Vec<&str> = input.lines().collect();
        assert!(detect_header_row(&lines));

        let input =
            "Name,Email,Phone\nJohn,john@example.com,555-1234\nMary,mary@example.com,555-5678";
        let lines: Vec<&str> = input.lines().collect();
        assert!(detect_header_row(&lines));

        let input = "ID,Code,Date\nA123,XY-789,2023-05-15\nB456,ZZ-123,2023-06-20";
        let lines: Vec<&str> = input.lines().collect();
        assert!(detect_header_row(&lines));
    }

    #[test]
    fn test_is_numeric() {
        assert!(is_numeric("123"));
        assert!(is_numeric("123.456"));
        assert!(is_numeric("-123"));
        assert!(is_numeric("+456"));
        assert!(is_numeric("123.456"));

        assert!(!is_numeric("name"));
        assert!(!is_numeric(""));
        assert!(!is_numeric("abc123"));
        assert!(!is_numeric("test@example.com"));
    }

    #[test]
    fn test_is_data_like() {
        assert!(is_data_like("123"));
        assert!(is_data_like("test@example.com"));
        assert!(is_data_like("http://example.com"));
        assert!(is_data_like("2023-05-15"));
        assert!(is_data_like("AB-123"));

        assert!(!is_data_like("first_name"));
        assert!(!is_data_like("name"));
        assert!(!is_data_like(""));
    }
}
