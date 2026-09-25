//! One path from input to records, for every format.

use std::fmt;
use std::io::{self, BufRead};
use tracing::debug;

use crate::csv_parser::CsvRecords;
use crate::detect::{Format, detect};
use crate::documents;
use crate::json_stream::JsonRecords;
use crate::lines::{Line, LineSource, UNDECODABLE};
use crate::parse::parse_logfmt;
use crate::record::Record;

/// Why reading a record failed.
#[derive(Debug)]
pub enum RecordError {
    /// The input could not be read.
    Io(io::Error),
    /// Input failed before anything parsed; nothing further is read.
    First {
        format: Format,
        line: usize,
        reason: String,
    },
    /// A record failed after others parsed; it is skipped.
    Skipped { line: usize, reason: String },
}

impl RecordError {
    /// A parse failure, fatal unless something has already parsed.
    pub(crate) fn parse(
        parsed_any: bool,
        format: Format,
        line: usize,
        reason: impl Into<String>,
    ) -> Self {
        let reason = reason.into();
        if parsed_any {
            RecordError::Skipped { line, reason }
        } else {
            RecordError::First {
                format,
                line,
                reason,
            }
        }
    }
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordError::Io(error) => write!(f, "{error}"),
            RecordError::First {
                format,
                line,
                reason,
            } => write!(f, "input is not {format}: line {line}: {reason}"),
            RecordError::Skipped { line, reason } => {
                write!(f, "failed to parse line {line}: {reason}")
            }
        }
    }
}

impl std::error::Error for RecordError {}

/// The records of one input source.
///
/// A forced format skips detection. A guessed YAML or TOML input whose first
/// document fails to parse is read as text lines.
pub struct Records<R: BufRead> {
    format: Format,
    reader: Reader<R>,
}

enum Reader<R: BufRead> {
    Json(JsonRecords<R>),
    Logfmt(LogfmtRecords<R>),
    Text(TextRecords<R>),
    Csv(CsvRecords<R>),
    Documents(std::vec::IntoIter<Result<Record, RecordError>>),
}

impl<R: BufRead> Records<R> {
    /// Choose the format of `reader`, or use `forced`, and start reading.
    pub fn open(reader: R, forced: Option<Format>) -> io::Result<Self> {
        let mut source = LineSource::new(reader);
        let (format, opening) = match forced {
            Some(format) => (format, None),
            None => {
                let detection = detect(&mut source)?;
                (detection.format, detection.opening)
            }
        };
        let chosen_by = match forced {
            Some(_) => "forced".to_string(),
            None => format!("precedence row {}", format.precedence_row()),
        };
        let records = match format {
            Format::Json => Self::with(format, Reader::Json(JsonRecords::new(source, opening))),
            Format::Logfmt => Self::with(format, Reader::Logfmt(LogfmtRecords::new(source))),
            Format::Text => Self::with(format, Reader::Text(TextRecords::new(source))),
            Format::Csv => Self::with(format, Reader::Csv(CsvRecords::new(source))),
            Format::Yaml | Format::Toml => {
                let lines = source.read_to_end()?;
                match documents::read(format, &lines, forced.is_some()) {
                    Some(parsed) => Self::with(format, Reader::Documents(parsed.into_iter())),
                    None => {
                        debug!("input format: text ({format} by {chosen_by} failed to parse)");
                        source.replay(lines);
                        return Ok(Self::with(
                            Format::Text,
                            Reader::Text(TextRecords::new(source)),
                        ));
                    }
                }
            }
        };
        debug!("input format: {format} ({chosen_by})");
        Ok(records)
    }

    fn with(format: Format, reader: Reader<R>) -> Self {
        Self { format, reader }
    }

    /// The format records are read as.
    pub fn format(&self) -> Format {
        self.format
    }
}

impl<R: BufRead> Iterator for Records<R> {
    type Item = Result<Record, RecordError>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.reader {
            Reader::Json(records) => records.next(),
            Reader::Logfmt(records) => records.next(),
            Reader::Text(records) => records.next(),
            Reader::Csv(records) => records.next(),
            Reader::Documents(records) => records.next(),
        }
    }
}

/// The failure for a line that is not valid UTF-8.
pub(crate) fn undecodable(parsed_any: bool, format: Format, line: &Line) -> RecordError {
    RecordError::parse(parsed_any, format, line.number, UNDECODABLE)
}

/// One record per non-blank line; leading `#` lines are records too.
struct TextRecords<R> {
    source: LineSource<R>,
    parsed_any: bool,
}

impl<R: BufRead> TextRecords<R> {
    fn new(source: LineSource<R>) -> Self {
        Self {
            source,
            parsed_any: false,
        }
    }
}

impl<R: BufRead> Iterator for TextRecords<R> {
    type Item = Result<Record, RecordError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.source.next_line() {
                Ok(Some(line)) if line.is_blank() => continue,
                Ok(Some(line)) if !line.decoded => {
                    let parsed_any = self.parsed_any;
                    return Some(Err(undecodable(parsed_any, Format::Text, &line)));
                }
                Ok(Some(line)) => {
                    self.parsed_any = true;
                    return Some(Ok(Record::text(line.text)));
                }
                Ok(None) => return None,
                Err(error) => return Some(Err(RecordError::Io(error))),
            }
        }
    }
}

/// One record per non-blank line; leading `#` lines are skipped.
struct LogfmtRecords<R> {
    source: LineSource<R>,
    parsed_any: bool,
}

impl<R: BufRead> LogfmtRecords<R> {
    fn new(source: LineSource<R>) -> Self {
        Self {
            source,
            parsed_any: false,
        }
    }
}

impl<R: BufRead> Iterator for LogfmtRecords<R> {
    type Item = Result<Record, RecordError>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = match self.source.next_content_line() {
            Ok(Some(line)) => line,
            Ok(None) => return None,
            Err(error) => return Some(Err(RecordError::Io(error))),
        };
        if !line.decoded {
            return Some(Err(undecodable(self.parsed_any, Format::Logfmt, &line)));
        }
        Some(match parse_logfmt(&line.text) {
            Some(value) => {
                self.parsed_any = true;
                Ok(Record::value(value, line.text))
            }
            None => Err(RecordError::parse(
                self.parsed_any,
                Format::Logfmt,
                line.number,
                "expected key=value pairs",
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn read(input: &str, forced: Option<Format>) -> (Format, Vec<Result<String, String>>) {
        let records = Records::open(Cursor::new(input.to_string()), forced).unwrap();
        let format = records.format();
        let sources = records
            .map(|item| match item {
                Ok(record) => Ok(record.source().to_string()),
                Err(error) => Err(error.to_string()),
            })
            .collect();
        (format, sources)
    }

    fn ok(sources: &[&str]) -> Vec<Result<String, String>> {
        sources.iter().map(|s| Ok(s.to_string())).collect()
    }

    #[test]
    fn guessed_yaml_that_fails_is_read_as_text() {
        let (format, sources) = read("INFO: started\nINFO: done", None);
        assert_eq!(format, Format::Text);
        assert_eq!(sources, ok(&["INFO: started", "INFO: done"]));
    }

    #[test]
    fn text_emits_leading_comments() {
        let (_, sources) = read("# note\n\nhello world", None);
        assert_eq!(sources, ok(&["# note", "hello world"]));
    }

    #[test]
    fn logfmt_skips_leading_comments_and_warns_on_later_failures() {
        let (format, sources) = read("# note\na=1\nnot logfmt\nb=2", None);
        assert_eq!(format, Format::Logfmt);
        assert_eq!(sources.len(), 3);
        assert_eq!(
            sources[1],
            Err("failed to parse line 3: expected key=value pairs".to_string())
        );
    }

    #[test]
    fn forced_format_skips_detection() {
        let (format, sources) = read("{\"a\":1}", Some(Format::Text));
        assert_eq!(format, Format::Text);
        assert_eq!(sources, ok(&["{\"a\":1}"]));
    }

    #[test]
    fn forced_format_first_failure_is_fatal() {
        let mut records = Records::open(Cursor::new("hello"), Some(Format::Logfmt)).unwrap();
        assert!(matches!(
            records.next(),
            Some(Err(RecordError::First { line: 1, .. }))
        ));
    }

    #[test]
    fn csv_header_row_is_not_a_record() {
        let (format, sources) = read("# c\nname,age\nTom,45\nAnn,30", None);
        assert_eq!(format, Format::Csv);
        assert_eq!(sources, ok(&["Tom,45", "Ann,30"]));
    }
}
