//! # parsm - **Parse 'Em** - Multi-Format Data Processor
//!
//! A powerful library for parsing, filtering, and transforming structured data from various formats.
//!
//! ## Overview
//!
//! `parsm` automatically detects and parses JSON, CSV, TOML, YAML, logfmt, and plain text,
//! providing powerful filtering and templating capabilities with a simple, intuitive syntax.
//!
//! ## Quick Start
//!
//! ```rust
//! use parsm::{parse_command, process_stream, StreamingParser};
//! use std::io::Cursor;
//!
//! // Parse a filter expression
//! let dsl = parse_command(r#"age > 25 {${name} is ${age} years old}"#)?;
//!
//! // Process streaming data
//! let input = r#"{"name": "Alice", "age": 30}"#;
//! let mut output = Vec::new();
//! process_stream(Cursor::new(input), &mut output)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Supported Formats
//!
//! - **JSON**: `{"name": "Alice", "age": 30}`
//! - **CSV**: `Alice,30,Engineer`
//! - **YAML**: `name: Alice\nage: 30`
//! - **TOML**: `name = "Alice"\nage = 30`
//! - **Logfmt**: `level=error msg="timeout" service=api`
//! - **Plain Text**: `Alice 30 Engineer`
//!
//! ## Filter Syntax
//!
//! - **Comparison**: `age > 25`, `name == "Alice"`
//! - **String ops**: `email ~ "@company.com"`, `name ^= "A"`, `file $= ".log"`
//! - **Boolean logic**: `age > 25 && active == true`, `!verified`
//! - **Nested fields**: `user.email == "alice@example.com"`
//!
//! ## Template Syntax
//!
//! Templates are enclosed in braces `{...}` and use `${variable}` or `$variable` for substitution:
//!
//! - **Field substitution**: `{${name} is ${age}}` or `{$name is ${age}}`
//! - **Indexed fields**: `{${1}, ${2}, ${3}}` (1-based, requires braces for numbers)
//! - **Original input**: `{${0}}` (entire original input, requires braces)
//! - **Nested fields**: `{${user.email}}` or `{$user.email}` (if unambiguous)
//! - **Mixed**: `{$name costs $$100}` (use $$ for literal dollar signs)
//!
//! ## Field Selection
//!
//! ```rust
//! use parsm::parse_command;
//!
//! // Extract specific fields using quoted syntax
//! let dsl = parse_command(r#""user.email""#)?;
//! assert!(dsl.field_selector.is_some());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Examples
//!
//! ### Basic Filtering
//!
//! ```rust
//! use parsm::{parse_command, FilterEngine};
//! use serde_json::json;
//!
//! let dsl = parse_command(r#"age > 25"#)?;
//! let data = json!({"name": "Alice", "age": 30});
//!
//! if let Some(filter) = &dsl.filter {
//!     let passes = FilterEngine::evaluate(filter, &data);
//!     assert!(passes);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Template Rendering
//!
//! ```rust
//! use parsm::parse_command;
//! use serde_json::json;
//!
//! let dsl = parse_command(r#"age > 25 {${name} is ${age} years old}"#)?;
//! let data = json!({"name": "Alice", "age": 30});
//!
//! if let Some(template) = &dsl.template {
//!     let output = template.render(&data);
//!     assert_eq!(output, "Alice is 30 years old");
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Format Detection and Parsing
//!
//! ```rust
//! use parsm::StreamingParser;
//!
//! // Create separate parsers for different formats
//! let mut json_parser = StreamingParser::new();
//! let json_result = json_parser.parse_line(r#"{"name": "Alice"}"#)?;
//!
//! let mut csv_parser = StreamingParser::new();
//! let csv_result = csv_parser.parse_line("Alice,30,Engineer")?;
//!
//! let mut logfmt_parser = StreamingParser::new();
//! let logfmt_result = logfmt_parser.parse_line("level=error msg=timeout")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Architecture
//!
//! The library consists of several key components:
//!
//! - [`parse`]: Multi-format parser with automatic detection
//! - [`filter`]: Boolean expression evaluation engine
//! - [`dsl`]: Domain-specific language parser using Pest
//! - High-level functions for stream processing
//!
//! ## Error Handling
//!
//! - **First line errors**: Fatal (format detection failure)
//! - **Subsequent errors**: Warnings with continued processing
//! - **Missing fields**: Graceful fallback behavior
//!
//! ## Performance
//!
//! - **Streaming**: Line-by-line processing for constant memory usage
//! - **Format detection**: Efficient with intelligent fallback
//! - **Large files**: Scales to gigabyte-scale data processing

pub mod dsl;
pub mod filter;
pub mod operators;
pub mod parse;

pub use dsl::{parse_command, DSLParser, ParsedDSL};
pub use filter::{ComparisonOp, FieldPath, FilterEngine, FilterExpr, FilterValue, Template};
pub use parse::{process_stream, Format, ParsedLine, StreamingParser};

use serde_json::Value;
use std::io::{BufRead, Write};

pub fn process_with_filter<R: BufRead, W: Write>(
    reader: R,
    mut writer: W,
    filter_expr: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let dsl = if let Some(expr) = filter_expr {
        parse_command(expr)?
    } else {
        ParsedDSL {
            filter: None,
            template: None,
            field_selector: None,
        }
    };

    let mut parser = StreamingParser::new();
    let mut line_count = 0;

    for line_result in reader.lines() {
        let line = line_result?;
        line_count += 1;

        if line.trim().is_empty() {
            continue;
        }

        match parser.parse_line(&line) {
            Ok(parsed_line) => {
                let json_value = convert_to_json(parsed_line, &line)?;

                let passes_filter = if let Some(ref filter) = dsl.filter {
                    FilterEngine::evaluate(filter, &json_value)
                } else {
                    true
                };

                if passes_filter {
                    let output = if let Some(ref template) = dsl.template {
                        template.render(&json_value)
                    } else {
                        serde_json::to_string(&json_value)?
                    };

                    writeln!(writer, "{}", output)?;
                }
            }
            Err(e) => {
                if line_count == 1 {
                    return Err(Box::new(e));
                } else {
                    eprintln!("Warning: Failed to parse line {}: {}", line_count, e);
                }
            }
        }
    }

    Ok(())
}

fn convert_to_json(
    parsed_line: ParsedLine,
    original_input: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let json_value = match parsed_line {
        ParsedLine::Json(mut val) => {
            if let Value::Object(ref mut obj) = val {
                obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            }
            val
        }
        ParsedLine::Csv(record) => {
            let mut obj = serde_json::Map::new();
            obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            for (i, field) in record.iter().enumerate() {
                obj.insert(format!("field_{}", i), Value::String(field.to_string()));
            }
            let values: Vec<Value> = record
                .iter()
                .map(|field| Value::String(field.to_string()))
                .collect();
            obj.insert("_array".to_string(), Value::Array(values));
            Value::Object(obj)
        }
        ParsedLine::Toml(val) => {
            let mut json_val = serde_json::to_value(val)?;
            if let Value::Object(ref mut obj) = json_val {
                obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            }
            json_val
        }
        ParsedLine::Yaml(val) => {
            let mut json_val = serde_json::to_value(val)?;
            if let Value::Object(ref mut obj) = json_val {
                obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            }
            json_val
        }
        ParsedLine::Logfmt(mut val) => {
            if let Value::Object(ref mut obj) = val {
                obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            }
            val
        }
        ParsedLine::Text(words) => {
            let mut obj = serde_json::Map::new();
            obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            for (i, word) in words.iter().enumerate() {
                obj.insert(format!("word_{}", i), Value::String(word.clone()));
            }
            let values: Vec<Value> = words.into_iter().map(Value::String).collect();
            obj.insert("_array".to_string(), Value::Array(values));
            Value::Object(obj)
        }
    };
    Ok(json_value)
}

pub fn detect_format(input: &str) -> Format {
    let mut parser = StreamingParser::new();
    if parser.parse_line(input).is_ok() {
        parser.get_format().unwrap_or(Format::Text)
    } else {
        Format::Text
    }
}

pub fn create_filter(expr: &str) -> Result<FilterExpr, Box<dyn std::error::Error>> {
    let dsl = parse_command(expr)?;
    dsl.filter
        .ok_or_else(|| "No filter expression found".into())
}

pub fn create_template(expr: &str) -> Result<Template, Box<dyn std::error::Error>> {
    let dsl = parse_command(expr)?;
    dsl.template
        .ok_or_else(|| "No template expression found".into())
}

/// Parse separate filter and template expressions
pub fn parse_separate_expressions(
    filter_input: Option<&str>,
    template_input: Option<&str>,
) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    DSLParser::parse_separate(filter_input, template_input).map_err(|e| e.into())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_end_to_end_json_processing() {
        let input = r#"{"name": "Alice", "age": 30}
{"name": "Bob", "age": 25}
{"name": "Charlie", "age": 35}"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_with_filter(reader, &mut output, Some(r#"age > 25 {${name} is ${age}}"#)).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Alice is 30");
        assert_eq!(lines[1], "Charlie is 35");
    }

    #[test]
    fn test_end_to_end_csv_processing() {
        let input = "Alice,30,Engineer\nBob,25,Designer\nCharlie,35,Manager";

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_with_filter(
            reader,
            &mut output,
            Some(r#"field_1 > "25" {${field_0}: ${field_2}}"#),
        )
        .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Alice: Engineer");
        assert_eq!(lines[1], "Charlie: Manager");
    }

    #[test]
    fn test_end_to_end_logfmt_processing() {
        let input = r#"level=info msg=\"Starting app\" service=web
level=error msg=\"DB connection failed\" service=api
level=info msg=\"Request processed\" service=web"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_with_filter(
            reader,
            &mut output,
            Some(r#"level == "error" {[ERROR] ${service}: ${msg}}"#),
        )
        .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "[ERROR] api: DB connection failed");
    }

    #[test]
    fn test_mixed_format_processing() {
        let formats = vec![
            (r#"{"name": "Alice", "type": "user"}"#, Format::Json),
            ("Alice,user,30", Format::Csv),
            ("level=info msg=test", Format::Logfmt),
            ("name: Alice", Format::Yaml),
        ];

        for (input, expected_format) in formats {
            let detected = detect_format(input);
            assert_eq!(
                detected, expected_format,
                "Failed to detect format for: {}",
                input
            );
        }
    }

    #[test]
    fn test_complex_filter_expressions() {
        let input = r#"{"name": "Alice", "age": 30, "active": true}
{"name": "Bob", "age": 25, "active": false}
{"name": "Charlie", "age": 35, "active": true}"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        let filter_expr = r#"(age > 25 && active == true) || name == "Bob""#;
        process_with_filter(reader, &mut output, Some(filter_expr)).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_utility_functions() {
        let filter = create_filter(r#"name == "Alice""#).unwrap();
        match filter {
            FilterExpr::Comparison { field, op, value } => {
                assert_eq!(field.parts, vec!["name"]);
                assert_eq!(op, ComparisonOp::Equal);
                assert_eq!(value, FilterValue::String("Alice".to_string()));
            }
            _ => panic!("Expected comparison"),
        }

        let template = create_template("{${name} is ${age}}").unwrap();
        assert_eq!(template.items.len(), 3);
    }

    #[test]
    fn test_no_filter_passthrough() {
        let input = r#"{"name": "Alice"}
{"name": "Bob"}"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        process_with_filter(reader, &mut output, None).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 2);
        let _: Value = serde_json::from_str(lines[0]).unwrap();
        let _: Value = serde_json::from_str(lines[1]).unwrap();
    }
}
