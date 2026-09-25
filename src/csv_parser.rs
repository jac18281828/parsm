//! CSV rows as records, keyed by header names when the input has a header
//! row.

use std::collections::VecDeque;
use std::io::{self, BufRead};
use std::sync::Arc;

use crate::detect::Format;
use crate::lines::{Line, LineSource};
use crate::parse::parse_csv_line;
use crate::record::{CsvHeader, Record};
use crate::records::RecordError;

/// Data rows sampled after the first row to decide whether it is a header.
const HEADER_SAMPLE_ROWS: usize = 5;

/// Records from CSV input. The first row and up to [`HEADER_SAMPLE_ROWS`]
/// data rows are buffered for header detection; later rows stream.
pub(crate) struct CsvRecords<R> {
    source: LineSource<R>,
    buffered: VecDeque<Line>,
    header: Option<Arc<CsvHeader>>,
    primed: bool,
    started: bool,
    parsed_any: bool,
}

impl<R: BufRead> CsvRecords<R> {
    pub(crate) fn new(source: LineSource<R>) -> Self {
        Self {
            source,
            buffered: VecDeque::new(),
            header: None,
            primed: false,
            started: false,
            parsed_any: false,
        }
    }

    /// The next row line; blank lines and leading `#` lines are skipped.
    fn next_row_line(&mut self) -> io::Result<Option<Line>> {
        while let Some(line) = self.source.next_line()? {
            if line.is_blank() || (!self.started && line.is_comment()) {
                continue;
            }
            self.started = true;
            return Ok(Some(line));
        }
        Ok(None)
    }

    fn prime(&mut self) -> io::Result<()> {
        self.primed = true;
        while self.buffered.len() <= HEADER_SAMPLE_ROWS {
            let Some(line) = self.next_row_line()? else {
                break;
            };
            self.buffered.push_back(line);
        }
        let sample: Vec<&str> = self.buffered.iter().map(|l| l.text.as_str()).collect();
        if !detect_header_row(&sample) {
            return Ok(());
        }
        let header = self
            .buffered
            .pop_front()
            .and_then(|line| parse_csv_line(&line.text));
        if let Some(fields) = header {
            self.header = Some(Arc::new(CsvHeader::new(&fields)));
            self.parsed_any = true;
        }
        Ok(())
    }

    fn next_line(&mut self) -> io::Result<Option<Line>> {
        if !self.primed {
            self.prime()?;
        }
        match self.buffered.pop_front() {
            Some(line) => Ok(Some(line)),
            None => self.next_row_line(),
        }
    }

    fn record(&mut self, line: Line) -> Result<Record, RecordError> {
        match parse_csv_line(&line.text) {
            Some(fields) => {
                self.parsed_any = true;
                Ok(Record::row(&fields, self.header.clone(), line.text))
            }
            None => Err(RecordError::parse(
                self.parsed_any,
                Format::Csv,
                line.number,
                "not a CSV row",
            )),
        }
    }
}

impl<R: BufRead> Iterator for CsvRecords<R> {
    type Item = Result<Record, RecordError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_line() {
            Ok(Some(line)) => Some(self.record(line)),
            Ok(None) => None,
            Err(error) => Some(Err(RecordError::Io(error))),
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
