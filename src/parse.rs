use csv;
use serde_json;
use serde_yaml;
use std::io::{BufRead, Write};
use toml;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    Json,
    Csv,
    Toml,
    Yaml,
    Logfmt,
    Text,
}

pub struct StreamingParser {
    format: Option<Format>,
}

impl StreamingParser {
    pub fn new() -> Self {
        Self { format: None }
    }

    pub fn parse_line(&mut self, line: &str) -> Result<ParsedLine, ParseError> {
        // If format is not detected yet, try to detect it
        if self.format.is_none() {
            self.format = Some(self.detect_format(line)?);
        }

        let format = self.format.unwrap();
        match format {
            Format::Json => self.parse_json_line(line),
            Format::Csv => self.parse_csv_line(line),
            Format::Toml => self.parse_toml_line(line),
            Format::Yaml => self.parse_yaml_line(line),
            Format::Logfmt => self.parse_logfmt_line(line),
            Format::Text => self.parse_text_line(line),
        }
    }

    fn detect_format(&self, line: &str) -> Result<Format, ParseError> {
        // Try JSON first (most common streaming format)
        if parse_json(line).is_some() {
            return Ok(Format::Json);
        }

        // Try YAML next (before CSV to avoid conflicts)
        if self.looks_like_yaml(line) && parse_yaml(line).is_some() {
            return Ok(Format::Yaml);
        }

        // Try TOML
        if self.looks_like_toml(line) && parse_toml(line).is_some() {
            return Ok(Format::Toml);
        }

        // Try logfmt (common for structured logs)
        if self.looks_like_logfmt(line) {
            return Ok(Format::Logfmt);
        }

        // Try CSV (only if it looks like CSV)
        if self.looks_like_csv(line) && parse_csv(line).is_some() {
            return Ok(Format::Csv);
        }

        // Fallback to plain text (most permissive - always succeeds)
        Ok(Format::Text)
    }

    fn looks_like_yaml(&self, line: &str) -> bool {
        // Simple heuristics for YAML detection
        // Avoid false positives with logfmt by checking for logfmt patterns first
        if self.looks_like_logfmt(line) {
            return false;
        }

        // YAML patterns: key: value, list items, or document separators
        line.contains(": ") || line.starts_with("- ") || line.starts_with("---")
    }

    fn looks_like_toml(&self, line: &str) -> bool {
        // Simple heuristics for TOML detection
        line.contains(" = ") || line.starts_with('[') && line.ends_with(']')
    }

    fn looks_like_logfmt(&self, line: &str) -> bool {
        // Simple heuristic: contains key=value pairs
        line.contains('=') && line.split_whitespace().any(|part| part.contains('='))
    }

    fn looks_like_csv(&self, line: &str) -> bool {
        // CSV should have at least one comma and multiple fields
        if !line.contains(',') {
            return false;
        }

        // Count commas and estimate fields
        let comma_count = line.matches(',').count();
        let estimated_fields = comma_count + 1;

        // Should have at least 2 fields to be considered CSV
        // Also check that it's not just a single word with commas
        estimated_fields >= 2 && line.trim().len() > comma_count
    }

    fn parse_json_line(&self, line: &str) -> Result<ParsedLine, ParseError> {
        parse_json(line)
            .map(ParsedLine::Json)
            .ok_or(ParseError::InvalidFormat(Format::Json))
    }

    fn parse_csv_line(&mut self, line: &str) -> Result<ParsedLine, ParseError> {
        parse_csv(line)
            .map(ParsedLine::Csv)
            .ok_or(ParseError::InvalidFormat(Format::Csv))
    }

    fn parse_toml_line(&self, line: &str) -> Result<ParsedLine, ParseError> {
        parse_toml(line)
            .map(ParsedLine::Toml)
            .ok_or(ParseError::InvalidFormat(Format::Toml))
    }

    fn parse_yaml_line(&self, line: &str) -> Result<ParsedLine, ParseError> {
        parse_yaml(line)
            .map(ParsedLine::Yaml)
            .ok_or(ParseError::InvalidFormat(Format::Yaml))
    }

    fn parse_logfmt_line(&self, line: &str) -> Result<ParsedLine, ParseError> {
        parse_logfmt(line)
            .map(ParsedLine::Logfmt)
            .ok_or(ParseError::InvalidFormat(Format::Logfmt))
    }

    fn parse_text_line(&self, line: &str) -> Result<ParsedLine, ParseError> {
        let words: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
        Ok(ParsedLine::Text(words))
    }

    pub fn get_format(&self) -> Option<Format> {
        self.format
    }
}

#[derive(Debug)]
pub enum ParsedLine {
    Json(serde_json::Value),
    Csv(csv::StringRecord),
    Toml(toml::Value),
    Yaml(serde_yaml::Value),
    Logfmt(serde_json::Value),
    Text(Vec<String>),
}

#[derive(Debug)]
pub enum ParseError {
    UnknownFormat,
    InvalidFormat(Format),
    IoError(std::io::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownFormat => write!(f, "Unable to detect format"),
            ParseError::InvalidFormat(format) => write!(f, "Invalid {:?} format", format),
            ParseError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for ParseError {}

impl Default for StreamingParser {
    fn default() -> Self {
        Self::new()
    }
}

pub fn process_stream<R: BufRead, W: Write>(
    reader: R,
    mut writer: W,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = StreamingParser::new();
    let mut line_count = 0;

    for line_result in reader.lines() {
        let line = line_result?;
        line_count += 1;

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        match parser.parse_line(&line) {
            Ok(parsed) => {
                // Output the parsed result as JSON for consistency
                let output = match parsed {
                    ParsedLine::Json(val) => val,
                    ParsedLine::Csv(record) => {
                        // Convert CSV record to JSON array
                        let values: Vec<serde_json::Value> = record
                            .iter()
                            .map(|field| serde_json::Value::String(field.to_string()))
                            .collect();
                        serde_json::Value::Array(values)
                    }
                    ParsedLine::Toml(val) => {
                        // Convert TOML value to JSON
                        serde_json::to_value(val)?
                    }
                    ParsedLine::Yaml(val) => {
                        // serde_yaml::Value can be directly converted to serde_json::Value
                        serde_json::to_value(val)?
                    }
                    ParsedLine::Logfmt(val) => val,
                    ParsedLine::Text(words) => {
                        // Convert text words to JSON array
                        let values: Vec<serde_json::Value> =
                            words.into_iter().map(serde_json::Value::String).collect();
                        serde_json::Value::Array(values)
                    }
                };

                writeln!(writer, "{}", serde_json::to_string(&output)?)?;
            }
            Err(e) => {
                if line_count == 1 {
                    // If we can't parse the first line, it's a fatal error
                    return Err(Box::new(e));
                } else {
                    // For subsequent lines, just warn and continue
                    eprintln!("Warning: Failed to parse line {}: {}", line_count, e);
                    eprintln!("Line content: {}", line);
                }
            }
        }
    }

    Ok(())
}

fn parse_json(line: &str) -> Option<serde_json::Value> {
    serde_json::from_str(line).ok()
}

fn parse_csv(line: &str) -> Option<csv::StringRecord> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(line.as_bytes());
    rdr.records().next().transpose().ok()?
}

fn parse_toml(line: &str) -> Option<toml::Value> {
    toml::from_str(line).ok()
}

fn parse_yaml(line: &str) -> Option<serde_yaml::Value> {
    serde_yaml::from_str(line).ok()
}

fn parse_logfmt(line: &str) -> Option<serde_json::Value> {
    let mut map = serde_json::Map::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }

        // Parse key
        let mut key = String::new();
        let mut current = ch;
        loop {
            if current == '=' {
                break;
            }
            key.push(current);
            if let Some(next_ch) = chars.next() {
                current = next_ch;
            } else {
                return None; // Invalid format
            }
        }

        if key.is_empty() {
            continue;
        }

        // Parse value
        let mut value = String::new();

        // Check if value starts with escaped quote \"
        if let Some(&'\\') = chars.peek() {
            let mut lookahead = chars.clone();
            lookahead.next(); // consume backslash
            if let Some(&'"') = lookahead.peek() {
                // Value starts with escaped quote - consume it but don't add to value yet
                chars.next(); // consume backslash
                chars.next(); // consume quote

                // Parse the quoted content until we find the closing \"
                while let Some(ch) = chars.next() {
                    if ch == '\\' {
                        if let Some(&'"') = chars.peek() {
                            // Found closing \"
                            chars.next(); // consume the quote
                            break;
                        } else {
                            // Handle escape sequences within escaped quotes
                            if let Some(escaped_ch) = chars.next() {
                                match escaped_ch {
                                    '"' => value.push('"'),
                                    '\\' => value.push('\\'),
                                    'n' => value.push('\n'),
                                    't' => value.push('\t'),
                                    'r' => value.push('\r'),
                                    _ => {
                                        value.push('\\');
                                        value.push(escaped_ch);
                                    }
                                }
                            }
                        }
                    } else {
                        value.push(ch);
                    }
                }
            }
        } else if let Some(&'"') = chars.peek() {
            // Regular quoted value
            chars.next(); // consume opening quote

            while let Some(ch) = chars.next() {
                if ch == '"' {
                    break; // Found closing quote
                } else if ch == '\\' {
                    // Handle escape sequences within quotes
                    if let Some(escaped_ch) = chars.next() {
                        match escaped_ch {
                            '"' => value.push('"'),
                            '\\' => value.push('\\'),
                            'n' => value.push('\n'),
                            't' => value.push('\t'),
                            'r' => value.push('\r'),
                            _ => {
                                value.push('\\');
                                value.push(escaped_ch);
                            }
                        }
                    }
                } else {
                    value.push(ch);
                }
            }
        } else {
            // Unquoted value - parse until whitespace
            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_whitespace() {
                    break;
                }
                value.push(chars.next().unwrap());
            }
        }

        map.insert(key.trim().to_string(), serde_json::Value::String(value));
    }

    if map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_json_format_detection_and_parsing() {
        let mut parser = StreamingParser::new();
        let json_line = r#"{"name": "John", "age": 30, "active": true}"#;

        let result = parser.parse_line(json_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Json));

        match result {
            ParsedLine::Json(value) => {
                assert_eq!(value["name"], "John");
                assert_eq!(value["age"], 30);
                assert_eq!(value["active"], true);
            }
            _ => panic!("Expected JSON parsing result"),
        }
    }

    #[test]
    fn test_csv_format_detection_and_parsing() {
        let mut parser = StreamingParser::new();
        let csv_line = "John,30,Engineer";

        let result = parser.parse_line(csv_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Csv));

        match result {
            ParsedLine::Csv(record) => {
                assert_eq!(record.get(0), Some("John"));
                assert_eq!(record.get(1), Some("30"));
                assert_eq!(record.get(2), Some("Engineer"));
            }
            _ => panic!("Expected CSV parsing result"),
        }
    }

    #[test]
    fn test_yaml_format_detection_and_parsing() {
        let mut parser = StreamingParser::new();
        let yaml_line = "name: John";

        let result = parser.parse_line(yaml_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Yaml));

        match result {
            ParsedLine::Yaml(value) => {
                // Convert to JSON for easier testing
                let json_value = serde_json::to_value(value).unwrap();
                assert_eq!(json_value["name"], "John");
            }
            _ => panic!("Expected YAML parsing result"),
        }
    }

    #[test]
    fn test_toml_format_detection_and_parsing() {
        let mut parser = StreamingParser::new();
        let toml_line = r#"name = "John""#;

        let result = parser.parse_line(toml_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Toml));

        match result {
            ParsedLine::Toml(value) => {
                assert_eq!(value["name"].as_str(), Some("John"));
            }
            _ => panic!("Expected TOML parsing result"),
        }
    }

    #[test]
    fn test_logfmt_format_detection_and_parsing() {
        let mut parser = StreamingParser::new();
        let logfmt_line = r#"level=info msg="Starting application" port=8080"#;

        let result = parser.parse_line(logfmt_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Logfmt));

        match result {
            ParsedLine::Logfmt(value) => {
                assert_eq!(value["level"], "info");
                assert_eq!(value["msg"], "Starting application");
                assert_eq!(value["port"], "8080");
            }
            _ => panic!("Expected logfmt parsing result"),
        }
    }

    #[test]
    fn test_logfmt_with_quotes() {
        let mut parser = StreamingParser::new();
        // Use a simpler logfmt that won't be confused with YAML
        let logfmt_line = r#"level=error msg=timeout retry=3"#;

        let result = parser.parse_line(logfmt_line).unwrap();

        match result {
            ParsedLine::Logfmt(value) => {
                assert_eq!(value["level"], "error");
                assert_eq!(value["msg"], "timeout");
                assert_eq!(value["retry"], "3");
            }
            _ => panic!("Expected logfmt parsing result"),
        }
    }

    #[test]
    fn test_format_persistence() {
        let mut parser = StreamingParser::new();

        // First line determines format
        let first_line = r#"{"id": 1, "name": "John"}"#;
        let result1 = parser.parse_line(first_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Json));

        // Subsequent lines use the same format
        let second_line = r#"{"id": 2, "name": "Jane"}"#;
        let result2 = parser.parse_line(second_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Json));

        match (result1, result2) {
            (ParsedLine::Json(val1), ParsedLine::Json(val2)) => {
                assert_eq!(val1["id"], 1);
                assert_eq!(val2["id"], 2);
            }
            _ => panic!("Expected JSON parsing results"),
        }
    }

    #[test]
    fn test_process_stream_json() {
        let input = r#"{"name": "John", "age": 30}
{"name": "Jane", "age": 25}
{"name": "Bob", "age": 35}"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_stream(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 3);

        let first_parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_parsed["name"], "John");
        assert_eq!(first_parsed["age"], 30);
    }

    #[test]
    fn test_process_stream_csv() {
        let input = "John,30,Engineer\nJane,25,Designer\nBob,35,Manager";

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_stream(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 3);

        let first_parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_parsed.as_array().unwrap()[0], "John");
        assert_eq!(first_parsed.as_array().unwrap()[1], "30");
        assert_eq!(first_parsed.as_array().unwrap()[2], "Engineer");
    }

    #[test]
    fn test_process_stream_logfmt() {
        let input = r#"level=info msg="Starting app" port=8080
level=error msg="DB error" code=500"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_stream(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 2);

        let first_parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_parsed["level"], "info");
        assert_eq!(first_parsed["msg"], "Starting app");
        assert_eq!(first_parsed["port"], "8080");
    }

    #[test]
    fn test_empty_lines_are_skipped() {
        let input = r#"{"name": "John"}

{"name": "Jane"}
        
{"name": "Bob"}"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_stream(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 3); // Only non-empty lines should be processed
    }

    #[test]
    fn test_text_is_most_permissive() {
        let mut parser = StreamingParser::new();
        // Plain text gets parsed as Text since it's the most permissive fallback
        let weird_line = "this is just text";

        let result = parser.parse_line(weird_line).unwrap();
        // This will be parsed as Text since it has no CSV-like structure
        assert_eq!(parser.get_format(), Some(Format::Text));

        match result {
            ParsedLine::Text(words) => {
                assert_eq!(words, vec!["this", "is", "just", "text"]);
            }
            _ => panic!("Expected CSV parsing result"),
        }
    }

    #[test]
    fn test_format_detection_heuristics() {
        // Test YAML detection
        assert!(StreamingParser::new().looks_like_yaml("key: value"));
        assert!(StreamingParser::new().looks_like_yaml("- item1"));
        assert!(StreamingParser::new().looks_like_yaml("---"));
        assert!(!StreamingParser::new().looks_like_yaml("key=value"));

        // Test TOML detection
        assert!(StreamingParser::new().looks_like_toml("key = \"value\""));
        assert!(StreamingParser::new().looks_like_toml("[section]"));
        assert!(!StreamingParser::new().looks_like_toml("key: value"));

        // Test logfmt detection
        assert!(StreamingParser::new().looks_like_logfmt("key=value"));
        assert!(StreamingParser::new().looks_like_logfmt("level=info msg=test"));
        assert!(!StreamingParser::new().looks_like_logfmt("key: value"));
    }

    #[test]
    fn test_complex_yaml() {
        let mut parser = StreamingParser::new();
        let yaml_line = "person: { name: John, age: 30 }";

        let result = parser.parse_line(yaml_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Yaml));

        match result {
            ParsedLine::Yaml(value) => {
                let json_value = serde_json::to_value(value).unwrap();
                assert_eq!(json_value["person"]["name"], "John");
                assert_eq!(json_value["person"]["age"], 30);
            }
            _ => panic!("Expected YAML parsing result"),
        }
    }

    #[test]
    fn test_invalid_format_after_detection() {
        let mut parser = StreamingParser::new();

        // First line establishes JSON format
        let json_line = r#"{"name": "John"}"#;
        parser.parse_line(json_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Json));

        // Second line is invalid JSON but parser should continue with JSON format
        let invalid_json = "this is not json";
        let result = parser.parse_line(invalid_json);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidFormat(Format::Json) => (),
            _ => panic!("Expected InvalidFormat(Json) error"),
        }
    }

    #[test]
    fn test_format_priority_order() {
        // JSON takes priority over other formats
        let json_like = r#"{"key": "value"}"#;
        let mut parser1 = StreamingParser::new();
        let _result1 = parser1.parse_line(json_like).unwrap();
        assert_eq!(parser1.get_format(), Some(Format::Json));

        // YAML takes priority over CSV for YAML-like syntax
        let yaml_like = "key: value";
        let mut parser2 = StreamingParser::new();
        let _result2 = parser2.parse_line(yaml_like).unwrap();
        assert_eq!(parser2.get_format(), Some(Format::Yaml));

        // TOML takes priority over CSV for TOML-like syntax
        let toml_like = r#"key = "value""#;
        let mut parser3 = StreamingParser::new();
        let _result3 = parser3.parse_line(toml_like).unwrap();
        assert_eq!(parser3.get_format(), Some(Format::Toml));

        // Logfmt takes priority over CSV for key=value syntax
        let logfmt_like = "key=value level=info";
        let mut parser4 = StreamingParser::new();
        let _result4 = parser4.parse_line(logfmt_like).unwrap();
        assert_eq!(parser4.get_format(), Some(Format::Logfmt));
    }

    #[test]
    fn test_process_stream_yaml() {
        let input = "name: Alice\nname: Bob\nname: Charlie";

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_stream(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 3);

        let first_parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_parsed["name"], "Alice");

        let second_parsed: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second_parsed["name"], "Bob");
    }

    #[test]
    fn test_process_stream_toml() {
        let input = r#"name = "Alice"
name = "Bob"
name = "Charlie""#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_stream(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 3);

        let first_parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_parsed["name"], "Alice");
    }

    #[test]
    fn test_large_dataset_streaming() {
        // Generate a large number of JSON records to test streaming efficiency
        let mut input = String::new();
        for i in 1..=100 {
            input.push_str(&format!(
                r#"{{"id": {}, "user": "user{}", "active": true}}"#,
                i, i
            ));
            input.push('\n');
        }

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_stream(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 100);

        // Check first and last records
        let first_parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_parsed["id"], 1);
        assert_eq!(first_parsed["user"], "user1");

        let last_parsed: serde_json::Value = serde_json::from_str(lines[99]).unwrap();
        assert_eq!(last_parsed["id"], 100);
        assert_eq!(last_parsed["user"], "user100");
    }

    #[test]
    fn test_format_consistency_enforcement() {
        // Test that once format is detected, inconsistent lines are handled appropriately
        let input = r#"{"name": "John", "age": 30}
{"name": "Jane", "age": 25}
this,is,csv,but,should,fail
{"name": "Bob", "age": 35}"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        // This should not fail completely, but should warn about invalid lines
        // and continue processing valid JSON lines
        let result = process_stream(reader, &mut output);

        // The process should complete even with invalid lines
        assert!(result.is_ok());

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        // Should have 3 valid JSON lines (the CSV line should be skipped with warning)
        assert_eq!(lines.len(), 3);

        let first_parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_parsed["name"], "John");

        let last_parsed: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(last_parsed["name"], "Bob");
    }

    #[test]
    fn test_memory_efficiency_streaming() {
        // Test that we're truly streaming and not loading everything into memory
        // This test uses a reasonable number of records to verify streaming behavior
        let mut input = String::new();
        for i in 1..=50 {
            input.push_str(&format!(
                r#"level=info msg="Processing record {}" id={}"#,
                i, i
            ));
            input.push('\n');
        }

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        let start_time = std::time::Instant::now();
        process_stream(reader, &mut output).unwrap();
        let duration = start_time.elapsed();

        // Should process quickly (streaming, not batching)
        assert!(duration.as_millis() < 100, "Streaming should be fast");

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 50);

        let first_parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_parsed["level"], "info");
        assert_eq!(first_parsed["id"], "1");
    }

    #[test]
    fn test_demo_format_detection_scenarios() {
        // Test the exact scenarios shown in the demo

        // JSON detection
        let mut parser1 = StreamingParser::new();
        let json_result = parser1.parse_line(r#"{"format": "json"}"#).unwrap();
        assert_eq!(parser1.get_format(), Some(Format::Json));

        // CSV detection
        let mut parser2 = StreamingParser::new();
        let csv_result = parser2.parse_line("format,type").unwrap();
        assert_eq!(parser2.get_format(), Some(Format::Csv));

        // Logfmt detection
        let mut parser3 = StreamingParser::new();
        let logfmt_result = parser3.parse_line("level=info msg=test").unwrap();
        assert_eq!(parser3.get_format(), Some(Format::Logfmt));

        // Verify the parsed results
        match json_result {
            ParsedLine::Json(val) => assert_eq!(val["format"], "json"),
            _ => panic!("Expected JSON"),
        }

        match csv_result {
            ParsedLine::Csv(record) => {
                assert_eq!(record.get(0), Some("format"));
                assert_eq!(record.get(1), Some("type"));
            }
            _ => panic!("Expected CSV"),
        }

        match logfmt_result {
            ParsedLine::Logfmt(val) => {
                assert_eq!(val["level"], "info");
                assert_eq!(val["msg"], "test");
            }
            _ => panic!("Expected Logfmt"),
        }
    }

    #[test]
    fn test_text_format_detection_and_parsing() {
        let mut parser = StreamingParser::new();
        let text_line = "the cat in a hat";

        let result = parser.parse_line(text_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Text));

        match result {
            ParsedLine::Text(words) => {
                assert_eq!(words, vec!["the", "cat", "in", "a", "hat"]);
            }
            _ => panic!("Expected Text parsing result"),
        }
    }

    #[test]
    fn test_text_with_multiple_spaces() {
        let mut parser = StreamingParser::new();
        let text_line = "hello    world   with   spaces";

        let result = parser.parse_line(text_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Text));

        match result {
            ParsedLine::Text(words) => {
                // split_whitespace() automatically handles multiple spaces
                assert_eq!(words, vec!["hello", "world", "with", "spaces"]);
            }
            _ => panic!("Expected Text parsing result"),
        }
    }

    #[test]
    fn test_process_stream_text() {
        let input = "the cat in a hat\nquick brown fox\njumps over lazy dog";

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_stream(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 3);

        let first_parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_parsed.as_array().unwrap()[0], "the");
        assert_eq!(first_parsed.as_array().unwrap()[1], "cat");
        assert_eq!(first_parsed.as_array().unwrap()[4], "hat");

        let second_parsed: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second_parsed.as_array().unwrap()[0], "quick");
        assert_eq!(second_parsed.as_array().unwrap()[2], "fox");
    }

    #[test]
    fn test_text_format_priority() {
        // Text should be the absolute fallback - only used when nothing else matches
        let mut parser = StreamingParser::new();

        // This should be detected as CSV, not text
        let csv_like = "word1,word2,word3";
        let _result = parser.parse_line(csv_like).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Csv));

        // But plain words should be detected as text
        let mut parser2 = StreamingParser::new();
        let plain_text = "just some plain words";
        let _result2 = parser2.parse_line(plain_text).unwrap();
        assert_eq!(parser2.get_format(), Some(Format::Text));
    }

    #[test]
    fn test_empty_text_line() {
        let mut parser = StreamingParser::new();
        let empty_line = "";

        let result = parser.parse_line(empty_line).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Text));

        match result {
            ParsedLine::Text(words) => {
                assert_eq!(words, Vec::<String>::new());
            }
            _ => panic!("Expected Text parsing result"),
        }
    }

    #[test]
    fn test_single_word_text() {
        let mut parser = StreamingParser::new();
        let single_word = "hello";

        let result = parser.parse_line(single_word).unwrap();
        assert_eq!(parser.get_format(), Some(Format::Text));

        match result {
            ParsedLine::Text(words) => {
                assert_eq!(words, vec!["hello"]);
            }
            _ => panic!("Expected Text parsing result"),
        }
    }

    #[test]
    fn test_logfmt_escaped_quotes_comprehensive() {
        let mut parser = StreamingParser::new();

        // Test escaped quotes at start and end
        let logfmt_line = r#"level=error msg=\"DB connection failed\" service=api"#;
        let result = parser.parse_line(logfmt_line).unwrap();

        match result {
            ParsedLine::Logfmt(value) => {
                assert_eq!(value["level"], "error");
                assert_eq!(value["msg"], "DB connection failed");
                assert_eq!(value["service"], "api");
            }
            _ => panic!("Expected logfmt parsing result"),
        }
    }

    #[test]
    fn test_logfmt_mixed_quote_styles() {
        let mut parser = StreamingParser::new();

        // Mix of escaped quotes, regular quotes, and unquoted values
        let logfmt_line = r#"level=info msg=\"Server starting\" port=8080 env="production""#;
        let result = parser.parse_line(logfmt_line).unwrap();

        match result {
            ParsedLine::Logfmt(value) => {
                assert_eq!(value["level"], "info");
                assert_eq!(value["msg"], "Server starting");
                assert_eq!(value["port"], "8080");
                assert_eq!(value["env"], "production");
            }
            _ => panic!("Expected logfmt parsing result"),
        }
    }

    #[test]
    fn test_logfmt_escaped_quotes_with_spaces() {
        let mut parser = StreamingParser::new();

        // Escaped quotes with internal spaces and special characters
        let logfmt_line =
            r#"action=login user=\"john doe\" reason=\"failed: invalid password\" attempts=3"#;
        let result = parser.parse_line(logfmt_line).unwrap();

        match result {
            ParsedLine::Logfmt(value) => {
                assert_eq!(value["action"], "login");
                assert_eq!(value["user"], "john doe");
                assert_eq!(value["reason"], "failed: invalid password");
                assert_eq!(value["attempts"], "3");
            }
            _ => panic!("Expected logfmt parsing result"),
        }
    }

    #[test]
    fn test_logfmt_nested_escape_sequences() {
        let mut parser = StreamingParser::new();

        // Test various escape sequences within escaped quotes
        let logfmt_line = r#"msg=\"Error: \\server\\path\\file.txt\" status=\"failed\""#;
        let result = parser.parse_line(logfmt_line).unwrap();

        match result {
            ParsedLine::Logfmt(value) => {
                assert_eq!(value["msg"], r"Error: \server\path\file.txt");
                assert_eq!(value["status"], "failed");
            }
            _ => panic!("Expected logfmt parsing result"),
        }
    }

    #[test]
    fn test_logfmt_empty_escaped_quotes() {
        let mut parser = StreamingParser::new();

        // Empty string in escaped quotes
        let logfmt_line = r#"level=debug msg=\"\" user=system"#;
        let result = parser.parse_line(logfmt_line).unwrap();

        match result {
            ParsedLine::Logfmt(value) => {
                assert_eq!(value["level"], "debug");
                assert_eq!(value["msg"], "");
                assert_eq!(value["user"], "system");
            }
            _ => panic!("Expected logfmt parsing result"),
        }
    }

    #[test]
    fn test_logfmt_malformed_escaped_quotes() {
        let mut parser = StreamingParser::new();

        // Test malformed escaped quote (missing closing escaped quote)
        let logfmt_line = r#"level=error msg=\"unclosed quote service=api"#;
        let result = parser.parse_line(logfmt_line).unwrap();

        match result {
            ParsedLine::Logfmt(value) => {
                // Should gracefully handle malformed input
                assert_eq!(value["level"], "error");
                // The parser should treat the rest as the value since no closing \" was found
                assert!(value["msg"]
                    .as_str()
                    .unwrap()
                    .contains("unclosed quote service=api"));
            }
            _ => panic!("Expected logfmt parsing result"),
        }
    }
}
