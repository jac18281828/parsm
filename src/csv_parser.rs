use crate::ParsedDSL;
/// CSV parsing module with header detection and field mapping
///
/// This module provides specialized CSV parsing that can:
/// - Detect header rows automatically by comparing field types
/// - Map header names to field names for easy access
/// - Fall back to indexed field names (field_0, field_1, etc.)
use serde_json::{Map, Value};
use std::io::Write;

/// Parse CSV document and process it
/// Returns true if parsing was successful, false otherwise
pub fn parse_csv_document(
    input: &str,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<bool, Box<dyn std::error::Error>> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return Ok(false);
    }

    let has_headers = lines.len() > 1 && detect_header_row(&lines);

    let mut rdr_no_headers = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(input.as_bytes());

    let header_names = if has_headers {
        parse_csv_header_names(lines[0])
    } else {
        Vec::new()
    };

    let mut records = Vec::new();

    for (line_idx, result) in rdr_no_headers.records().enumerate() {
        let record = match result {
            Ok(record) => record,
            Err(_) => continue,
        };

        let mut obj = Map::new();

        let original_line_value = if let Some(original_line) = lines.get(line_idx) {
            original_line.to_string()
        } else {
            input.trim().to_string()
        };

        obj.insert("0".to_string(), Value::String(original_line_value));

        for (i, field) in record.iter().enumerate() {
            let field_value = field.to_string();
            let index = i + 1;

            obj.insert(index.to_string(), Value::String(field_value.clone()));

            let field_name = format!("field_{i}");
            obj.insert(field_name.clone(), Value::String(field_value.clone()));
        }

        if let Some(original_line) = lines.get(line_idx) {
            obj.insert("$0".to_string(), Value::String(original_line.to_string()));
            obj.insert("${0}".to_string(), Value::String(original_line.to_string()));
        }

        if has_headers && line_idx > 0 {
            for (i, field) in record.iter().enumerate() {
                if let Some(header_name) = header_names.get(i) {
                    let field_value = field.to_string();
                    let header_name_lowercase = header_name.to_lowercase();

                    obj.insert(header_name.clone(), Value::String(field_value.clone()));
                    if header_name.to_lowercase() != *header_name {
                        obj.insert(
                            header_name_lowercase.clone(),
                            Value::String(field_value.clone()),
                        );
                    }

                    obj.insert(header_name.clone(), Value::String(field_value.clone()));
                    obj.insert(
                        format!("${header_name}"),
                        Value::String(field_value.clone()),
                    );
                    obj.insert(
                        format!("${{{header_name}}}"),
                        Value::String(field_value.clone()),
                    );
                }
            }
        }

        let values: Vec<Value> = record
            .iter()
            .map(|field| Value::String(field.to_string()))
            .collect();
        obj.insert("_array".to_string(), Value::Array(values));

        records.push(Value::Object(obj));
    }

    if records.is_empty() {
        return Ok(false);
    }

    let records_to_process = if has_headers && !records.is_empty() {
        &records[1..]
    } else {
        &records[..]
    };

    for record in records_to_process {
        if let Some(ref field_selector) = dsl.field_selector {
            if let Some(extracted) = field_selector.extract_field(record) {
                writeln!(writer, "{extracted}")?;
            }
        } else {
            crate::process_single_value(record, dsl, writer)?;
        }
    }

    Ok(true)
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

/// Parses a single CSV line into fields.
fn parse_csv_line(line: &str) -> Option<csv::StringRecord> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(line.as_bytes());
    rdr.records().next().transpose().ok()?
}

/// Parses header names from a CSV line, returning them as lowercase strings.
fn parse_csv_header_names(line: &str) -> Vec<String> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(line.as_bytes());

    if let Ok(Some(record)) = rdr.records().next().transpose() {
        record
            .iter()
            .map(|field| field.trim().to_lowercase())
            .collect()
    } else {
        Vec::new()
    }
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
