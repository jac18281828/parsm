use crate::ParsedDSL;
/// CSV parsing module with header detection and field mapping
///
/// This module provides specialized CSV parsing that can:
/// - Detect header rows automatically by comparing field types
/// - Map header names to field names for easy access
/// - Fall back to indexed field names (field_0, field_1, etc.)
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::io::Write;

/// Parse CSV document and process it
/// Returns true if parsing was successful, false otherwise
pub fn parse_csv_document(
    input: &str,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<bool, Box<dyn std::error::Error>> {
    // First try parsing without headers to capture all rows as data
    let mut rdr_no_headers = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(input.as_bytes());

    let mut records = Vec::new();
    let lines: Vec<&str> = input.lines().collect();

    // Detect if first row looks like headers (non-numeric, reasonable field names)
    let has_headers = if lines.len() >= 2 {
        detect_csv_headers_improved(input)
    } else {
        false
    };

    // Extract header names if detected
    let header_names = if has_headers {
        if let Some(first_line) = lines.first() {
            parse_csv_header_names(first_line)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    for (line_idx, result) in rdr_no_headers.records().enumerate() {
        let record = match result {
            Ok(record) => record,
            Err(_) => continue,
        };

        let mut obj = Map::new();

        // Add original line (use the specific line, not the entire input)
        if let Some(original_line) = lines.get(line_idx) {
            obj.insert("0".to_string(), Value::String(original_line.to_string()));
        } else {
            obj.insert("0".to_string(), Value::String(input.trim().to_string()));
        }

        // Add indexed fields (field_0, field_1, etc.)
        for (i, field) in record.iter().enumerate() {
            obj.insert(format!("field_{i}"), Value::String(field.to_string()));
        }

        // Add named fields from headers if detected and this is not the header row
        if has_headers && line_idx > 0 {
            for (i, field) in record.iter().enumerate() {
                if let Some(header_name) = header_names.get(i) {
                    obj.insert(header_name.clone(), Value::String(field.to_string()));
                }
            }
        }

        // Add array representation
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

    // Process each record (skip header row if detected)
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
            process_single_value(record, dsl, writer)?;
        }
    }

    Ok(true)
}

/// Process a single value with filter and template/field selector
fn process_single_value(
    value: &serde_json::Value,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::FilterEngine;

    let passes_filter = if let Some(ref filter) = dsl.filter {
        FilterEngine::evaluate(filter, value)
    } else {
        true
    };

    if passes_filter {
        let output = if let Some(ref template) = dsl.template {
            template.render(value)
        } else {
            serde_json::to_string(value)?
        };
        writeln!(writer, "{output}")?;
    }
    Ok(())
}

/// Improved header detection using multiple heuristics
#[derive(Debug, Default)]
struct HeaderScore {
    type_consistency: f32,  // How consistent are types within columns
    type_difference: f32,   // How different are types between row 0 and row 1
    text_ratio: f32,        // Ratio of text fields in row 0
    pattern_score: f32,     // Header-like patterns (snake_case, etc.)
    uniqueness: f32,        // Are row 0 values unique (headers usually are)
    statistical_score: f32, // Statistical properties of the data
}

impl HeaderScore {
    fn total(&self) -> f32 {
        // If we found non-text fields in the first row, heavily penalize
        if self.type_difference == 0.0 && self.text_ratio < 1.0 {
            return 0.0; // Definitely not headers
        }

        // If first row has empty fields, it's unlikely to be headers
        if self.text_ratio < 0.7 {
            // More than 30% non-text fields
            return 0.0;
        }

        self.type_consistency * 0.25
            + self.type_difference * 0.25
            + self.text_ratio * 0.15
            + self.pattern_score * 0.15
            + self.uniqueness * 0.10
            + self.statistical_score * 0.10
    }
}

/// Detect headers using multiple heuristics
pub fn detect_csv_headers_improved(input: &str) -> bool {
    let lines: Vec<&str> = input.lines().collect();

    // Need at least 2 rows for header detection
    if lines.len() < 2 {
        return false;
    }

    // For small files (≤100 lines), use simple header check
    if lines.len() <= 100 {
        return simple_header_check_any_rows(&lines);
    }

    // For large files (>100 lines), use statistical check with sampling
    detect_headers_with_sampling(input)
}

fn calculate_header_score(records: &[csv::StringRecord]) -> HeaderScore {
    let mut score = HeaderScore::default();

    if records.is_empty() {
        return score;
    }

    let first_row = &records[0];
    let num_columns = first_row.len();

    // 1. Type consistency within columns (excluding first row)
    let mut consistent_columns = 0;
    for col_idx in 0..num_columns {
        let types: Vec<_> = records[1..]
            .iter()
            .filter_map(|row| row.get(col_idx))
            .map(|field| classify_field_type(field.trim()))
            .collect();

        if types.len() > 1 {
            let first_type = &types[0];
            let consistency =
                types.iter().filter(|t| *t == first_type).count() as f32 / types.len() as f32;

            if consistency > 0.8 {
                consistent_columns += 1;
            }
        }
    }
    score.type_consistency = consistent_columns as f32 / num_columns as f32;

    // 2. Type difference between row 0 and rest
    let mut type_differences = 0;
    let mut non_text_in_first_row = 0;

    for col_idx in 0..num_columns {
        if let Some(header_field) = first_row.get(col_idx) {
            let header_type = classify_field_type(header_field.trim());

            // Count non-text fields in first row (these are likely NOT headers)
            if header_type != FieldType::Text && header_type != FieldType::Empty {
                non_text_in_first_row += 1;
            }

            // Check type difference with data rows
            if let Some(data_field) = records.get(1).and_then(|r| r.get(col_idx)) {
                let data_type = classify_field_type(data_field.trim());

                if header_type != data_type && header_type == FieldType::Text {
                    type_differences += 1;
                }
            }
        }
    }

    // Penalize if first row has non-text fields (like numbers)
    if non_text_in_first_row > 0 {
        score.type_difference = 0.0; // Strong signal that it's NOT a header
    } else {
        score.type_difference = type_differences as f32 / num_columns as f32;
    }

    // 3. Text ratio in first row
    let text_fields = first_row
        .iter()
        .filter(|field| classify_field_type(field.trim()) == FieldType::Text)
        .count();
    score.text_ratio = text_fields as f32 / num_columns as f32;

    // 4. Pattern score (header-like patterns)
    let pattern_matches = first_row
        .iter()
        .filter(|field| has_header_pattern(field.trim()))
        .count();
    score.pattern_score = pattern_matches as f32 / num_columns as f32;

    // 5. Uniqueness of first row values
    let unique_values: HashSet<_> = first_row.iter().map(|f| f.trim().to_lowercase()).collect();
    score.uniqueness = unique_values.len() as f32 / num_columns as f32;

    // 6. Statistical properties
    score.statistical_score = calculate_statistical_score(first_row, &records[1..]);

    score
}

/// Check for header-like patterns without hardcoded list
fn has_header_pattern(field: &str) -> bool {
    let field = field.trim();

    // Empty or too short
    if field.len() < 2 {
        return false;
    }

    // Strong header indicators: spaces and underscores
    let has_underscore = field.contains('_');
    let has_space = field.contains(' ');

    // If it has spaces or underscores, it's very likely a header
    if has_underscore || has_space {
        // But make sure it's not obviously data
        let no_obvious_data_patterns = !field.contains('@')  // email
            && !field.contains('/')  // date
            && !field.contains(':')  // time
            && !field.chars().all(|c| c.is_numeric() || c == '.' || c == '-' || c == ' '); // number with spaces

        return no_obvious_data_patterns;
    }

    // Check for other common patterns
    let has_camel_case =
        field.chars().any(|c| c.is_uppercase()) && field.chars().any(|c| c.is_lowercase());
    let is_all_caps = field
        .chars()
        .all(|c| !c.is_alphabetic() || c.is_uppercase());

    // Check for word-like structure (not data)
    let is_word_like =
        field.chars().filter(|c| c.is_alphabetic()).count() as f32 / field.len() as f32 > 0.5;

    // No special data patterns
    let no_data_patterns = !field.contains('@')  // email
        && !field.contains('/')  // date
        && !field.contains(':')  // time
        && !field.chars().all(|c| c.is_numeric() || c == '.' || c == '-'); // number

    is_word_like && no_data_patterns && (has_camel_case || is_all_caps)
}

/// Calculate statistical properties that distinguish headers from data
fn calculate_statistical_score(
    first_row: &csv::StringRecord,
    data_rows: &[csv::StringRecord],
) -> f32 {
    if data_rows.is_empty() {
        return 0.0;
    }

    let mut scores = Vec::new();

    for col_idx in 0..first_row.len() {
        if let Some(header) = first_row.get(col_idx) {
            let header = header.trim().to_lowercase();

            // Check if header appears in the data (unlikely for real headers)
            let appears_in_data = data_rows.iter().any(|row| {
                row.get(col_idx)
                    .map(|f| f.trim().to_lowercase() == header)
                    .unwrap_or(false)
            });

            if !appears_in_data {
                scores.push(1.0);
            } else {
                scores.push(0.0);
            }
        }
    }

    if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f32>() / scores.len() as f32
    }
}

/// Simple header check for any number of rows (for small files ≤100 lines)
/// This is biased toward detecting headers when the first row looks like header names
fn simple_header_check_any_rows(lines: &[&str]) -> bool {
    if lines.len() < 2 {
        return false;
    }

    if let (Some(row0), Some(row1)) = (parse_csv_line(lines[0]), parse_csv_line(lines[1])) {
        // All fields in row 0 should be text (basic requirement)
        let all_text_row0 = row0
            .iter()
            .all(|f| classify_field_type(f.trim()) == FieldType::Text);

        if !all_text_row0 {
            return false; // First row has numbers/emails/etc - definitely not headers
        }

        // Check if first row has header-like patterns
        let header_pattern_count = row0
            .iter()
            .filter(|field| has_header_pattern(field.trim()))
            .count();

        let header_pattern_ratio = header_pattern_count as f32 / row0.len() as f32;

        // If we have good header patterns, bias toward headers
        if header_pattern_ratio >= 0.5 {
            return true; // Strong signal of headers
        }

        // Check for type differences between row 0 and row 1
        let has_type_difference = row0.iter().zip(row1.iter()).any(|(h, d)| {
            let header_type = classify_field_type(h.trim());
            let data_type = classify_field_type(d.trim());
            header_type != data_type
        });

        // If there are type differences, likely headers
        if has_type_difference {
            return true;
        }

        // Check if first row values are unique (headers usually are)
        let unique_values: HashSet<_> = row0.iter().map(|f| f.trim().to_lowercase()).collect();
        let uniqueness_ratio = unique_values.len() as f32 / row0.len() as f32;

        // If most values in first row are unique, likely headers
        if uniqueness_ratio >= 0.8 {
            return true;
        }

        // Check if first row values appear in data rows (unlikely for headers)
        let first_row_values: HashSet<_> = row0.iter().map(|f| f.trim().to_lowercase()).collect();
        let appears_in_data = lines[1..].iter().any(|line| {
            if let Some(data_row) = parse_csv_line(line) {
                data_row
                    .iter()
                    .any(|field| first_row_values.contains(&field.trim().to_lowercase()))
            } else {
                false
            }
        });

        // If first row values don't appear in data, likely headers
        !appears_in_data
    } else {
        false
    }
}

/// Detect headers with sampling for large files (>100 lines)
fn detect_headers_with_sampling(input: &str) -> bool {
    let lines: Vec<&str> = input.lines().collect();

    if lines.len() < 2 {
        return false;
    }

    // Sample up to 100 lines for statistical analysis
    let sample_size = std::cmp::min(100, lines.len());
    let sampled_lines = &lines[0..sample_size];

    // Parse the sampled lines into CSV records
    let records: Vec<_> = sampled_lines
        .iter()
        .filter_map(|line| parse_csv_line(line))
        .collect();

    if records.len() < 2 {
        return false;
    }

    // Calculate header score using statistical analysis
    let score = calculate_header_score(&records);
    score.total() > 0.6 // Threshold for header detection
}

#[derive(Debug, PartialEq, Clone)]
enum FieldType {
    Text,
    Number,
    Email,
    Date,
    Boolean,
    Empty,
}

/// Classify the type of a field based on its content
fn classify_field_type(field: &str) -> FieldType {
    let field = field.trim();

    if field.is_empty() {
        return FieldType::Empty;
    }

    // Check for email
    if field.contains('@') && field.contains('.') {
        return FieldType::Email;
    }

    // Check for boolean
    let lower = field.to_lowercase();
    if lower == "true"
        || lower == "false"
        || lower == "yes"
        || lower == "no"
        || lower == "y"
        || lower == "n"
        || lower == "1"
        || lower == "0"
    {
        return FieldType::Boolean;
    }

    // Check for number (including decimals, negatives)
    if field
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+')
        && field.chars().any(|c| c.is_ascii_digit())
    {
        return FieldType::Number;
    }

    // Check for date patterns (basic check)
    if field.contains('/') || field.contains('-') {
        let parts: Vec<&str> = if field.contains('/') {
            field.split('/').collect()
        } else {
            field.split('-').collect()
        };

        if parts.len() == 3
            && parts
                .iter()
                .all(|part| part.chars().all(|c| c.is_ascii_digit()))
        {
            return FieldType::Date;
        }
    }

    // Default to text
    FieldType::Text
}

/// Parse a single CSV line into fields
fn parse_csv_line(line: &str) -> Option<csv::StringRecord> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(line.as_bytes());
    rdr.records().next().transpose().ok()?
}

/// Parse header names from a CSV line
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
        assert!(detect_csv_headers_improved(input));
    }

    #[test]
    fn test_header_detection_no_headers() {
        let input = "Tom,45,engineer\nAlice,30,doctor";
        assert!(!detect_csv_headers_improved(input));
    }

    #[test]
    fn test_no_header_with_mixed_types() {
        // This should NOT be detected as having headers
        let input = "Alice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager";
        assert!(!detect_csv_headers_improved(input));
    }

    #[test]
    fn test_header_detection_all_text_headers() {
        // This SHOULD be detected as having headers
        let input = "first_name,last_name,job_title\nAlice,Smith,Engineer\nBob,Jones,Designer";
        assert!(detect_csv_headers_improved(input));
    }

    #[test]
    fn test_field_type_classification() {
        assert_eq!(classify_field_type("name"), FieldType::Text);
        assert_eq!(classify_field_type("123"), FieldType::Number);
        assert_eq!(classify_field_type("test@example.com"), FieldType::Email);
        assert_eq!(classify_field_type("true"), FieldType::Boolean);
        assert_eq!(classify_field_type("2023/12/25"), FieldType::Date);
    }

    #[test]
    fn test_header_pattern_detection() {
        // Positive cases - strong header indicators
        assert!(has_header_pattern("user_id"));
        assert!(has_header_pattern("firstName"));
        assert!(has_header_pattern("USER_NAME"));
        assert!(has_header_pattern("first name")); // space is strong indicator
        assert!(has_header_pattern("last name"));
        assert!(has_header_pattern("job title"));
        assert!(has_header_pattern("email address"));
        assert!(has_header_pattern("phone_number"));
        assert!(has_header_pattern("date of birth"));

        // Negative cases
        assert!(!has_header_pattern("test@example.com"));
        assert!(!has_header_pattern("123"));
        assert!(!has_header_pattern("2023-12-25"));
        assert!(!has_header_pattern("a")); // too short
        assert!(!has_header_pattern("123 456")); // numbers with space
        assert!(!has_header_pattern("12:34:56")); // time
        assert!(!has_header_pattern("path/to/file")); // path
    }

    #[test]
    fn test_header_score_calculation() {
        let input =
            "id,name,email,age\n1,John Doe,john@example.com,30\n2,Jane Smith,jane@example.com,25";
        let lines: Vec<&str> = input.lines().collect();
        let records: Vec<_> = lines
            .iter()
            .filter_map(|line| parse_csv_line(line))
            .collect();

        let score = calculate_header_score(&records);
        assert!(score.total() > 0.5);
    }

    #[test]
    fn test_simple_header_check_any_rows() {
        // Test with 2 rows (headers detected)
        let lines = vec!["name,age", "John,30"];
        assert!(simple_header_check_any_rows(&lines));

        // Test with 3 rows (headers detected)
        let lines = vec!["name,age", "John,30", "Jane,25"];
        assert!(simple_header_check_any_rows(&lines));

        // Test without headers
        let lines = vec!["John,30", "Jane,25"];
        assert!(!simple_header_check_any_rows(&lines));

        // Test with mixed types in first row (no headers)
        let lines = vec!["Alice,30", "Bob,25"];
        assert!(!simple_header_check_any_rows(&lines));
    }

    #[test]
    fn test_detect_headers_with_sampling() {
        // Create a large CSV (>100 lines) with headers
        let mut large_csv = String::from("id,name,age,email\n");
        for i in 1..=150 {
            large_csv.push_str(&format!(
                "{},Person{},{},person{}@example.com\n",
                i,
                i,
                20 + (i % 50),
                i
            ));
        }

        assert!(detect_headers_with_sampling(&large_csv));

        // Create a large CSV without headers
        let mut large_csv_no_headers = String::new();
        for i in 1..=150 {
            large_csv_no_headers.push_str(&format!(
                "{},Person{},{},person{}@example.com\n",
                i,
                i,
                20 + (i % 50),
                i
            ));
        }

        assert!(!detect_headers_with_sampling(&large_csv_no_headers));
    }

    #[test]
    fn test_header_detection_small_vs_large_files() {
        // Small file (≤100 lines) - should use simple check
        let small_csv = "name,age\nJohn,30\nJane,25";
        assert!(detect_csv_headers_improved(small_csv));

        // Large file (>100 lines) - should use sampling
        let mut large_csv = String::from("id,name,age\n");
        for i in 1..=150 {
            large_csv.push_str(&format!("{},Person{},{}\n", i, i, 20 + (i % 50)));
        }
        assert!(detect_csv_headers_improved(&large_csv));
    }

    #[test]
    fn test_statistical_score() {
        let header = parse_csv_line("id,name,value").unwrap();
        let data = vec![
            parse_csv_line("1,John,100").unwrap(),
            parse_csv_line("2,Jane,200").unwrap(),
        ];

        let score = calculate_statistical_score(&header, &data);
        assert_eq!(score, 1.0); // Headers don't appear in data
    }

    #[test]
    fn debug_header_detection_all_text_headers() {
        let input = "first_name,last_name,job_title\nAlice,Smith,Engineer\nBob,Jones,Designer";
        let lines: Vec<&str> = input.lines().collect();

        println!("Lines: {lines:?}");

        let row0 = parse_csv_line(lines[0]).unwrap();
        let row1 = parse_csv_line(lines[1]).unwrap();

        println!("Row0: {:?}", row0.iter().collect::<Vec<_>>());
        println!("Row1: {:?}", row1.iter().collect::<Vec<_>>());

        for (i, field) in row0.iter().enumerate() {
            let field_type = classify_field_type(field.trim());
            println!("Row0[{i}]: '{field}' -> {field_type:?}");
        }

        for (i, field) in row1.iter().enumerate() {
            let field_type = classify_field_type(field.trim());
            println!("Row1[{i}]: '{field}' -> {field_type:?}");
        }

        let all_text = row0
            .iter()
            .all(|f| classify_field_type(f.trim()) == FieldType::Text);
        let has_non_text = row1
            .iter()
            .any(|f| classify_field_type(f.trim()) != FieldType::Text);

        println!("All text in row0: {all_text}");
        println!("Has non-text in row1: {has_non_text}");

        let result = simple_header_check_any_rows(&lines);
        println!("Simple header check result: {result}");
    }
}
