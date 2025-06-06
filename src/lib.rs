// lib.rs - Library structure and public API

pub mod dsl;
pub mod filter;
pub mod parse;

pub use dsl::{parse_command, DSLParser, ParsedDSL};
pub use filter::{ComparisonOp, FieldPath, FilterEngine, FilterExpr, FilterValue, Template};
pub use parse::{process_stream, Format, ParsedLine, StreamingParser};

use serde_json::Value;
use std::io::{BufRead, Write};

/// High-level API for processing streams with filters and templates
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

                // Apply filter
                let passes_filter = if let Some(ref filter) = dsl.filter {
                    FilterEngine::evaluate(filter, &json_value)
                } else {
                    true
                };

                if passes_filter {
                    // Apply template or output JSON
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

/// Convert ParsedLine to JSON Value for uniform processing
fn convert_to_json(parsed_line: ParsedLine, original_input: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let json_value = match parsed_line {
        ParsedLine::Json(mut val) => {
            // Add original input for $$ template
            if let Value::Object(ref mut obj) = val {
                obj.insert("$$".to_string(), Value::String(original_input.to_string()));
            }
            val
        }
        ParsedLine::Csv(record) => {
            let mut obj = serde_json::Map::new();
            // Add the entire input line for $$ template
            obj.insert("$$".to_string(), Value::String(original_input.to_string()));
            // Add indexed fields
            for (i, field) in record.iter().enumerate() {
                obj.insert(format!("field_{}", i), Value::String(field.to_string()));
            }
            // Add array representation
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
                obj.insert("$$".to_string(), Value::String(original_input.to_string()));
            }
            json_val
        }
        ParsedLine::Yaml(val) => {
            let mut json_val = serde_json::to_value(val)?;
            if let Value::Object(ref mut obj) = json_val {
                obj.insert("$$".to_string(), Value::String(original_input.to_string()));
            }
            json_val
        }
        ParsedLine::Logfmt(mut val) => {
            if let Value::Object(ref mut obj) = val {
                obj.insert("$$".to_string(), Value::String(original_input.to_string()));
            }
            val
        }
        ParsedLine::Text(words) => {
            let mut obj = serde_json::Map::new();
            // Add the entire input line for $$ template
            obj.insert("$$".to_string(), Value::String(original_input.to_string()));
            // Add indexed words
            for (i, word) in words.iter().enumerate() {
                obj.insert(format!("word_{}", i), Value::String(word.clone()));
            }
            // Add array representation
            let values: Vec<Value> = words.into_iter().map(Value::String).collect();
            obj.insert("_array".to_string(), Value::Array(values));
            Value::Object(obj)
        }
    };
    Ok(json_value)
}

/// Utility function to detect what format a string is
pub fn detect_format(input: &str) -> Format {
    let mut parser = StreamingParser::new();
    // Try to parse the first line to determine format
    if let Ok(_) = parser.parse_line(input) {
        parser.get_format().unwrap_or(Format::Text)
    } else {
        Format::Text
    }
}

/// Create a filter expression from a string
pub fn create_filter(expr: &str) -> Result<FilterExpr, Box<dyn std::error::Error>> {
    let dsl = parse_command(expr)?;
    dsl.filter
        .ok_or_else(|| "No filter expression found".into())
}

/// Create a template from a string  
pub fn create_template(expr: &str) -> Result<Template, Box<dyn std::error::Error>> {
    let dsl = parse_command(&format!("true {}", expr))?; // Add dummy filter
    dsl.template
        .ok_or_else(|| "No template expression found".into())
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

        // Filter for people over 25 and format output
        process_with_filter(reader, &mut output, Some(r#"age > 25 $name is $age"#)).unwrap();

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

        // Filter CSV and format
        process_with_filter(
            reader,
            &mut output,
            Some(r#"field_1 > "25" $1: $3"#),
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
        let input = r#"level=info msg="Starting app" service=web
level=error msg="DB connection failed" service=api
level=info msg="Request processed" service=web"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        // Filter for errors and format
        process_with_filter(
            reader,
            &mut output,
            Some(r#"level == "error" [ERROR] $service: $msg"#),
        )
        .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "[ERROR] api: DB connection failed");
    }

    #[test]
    fn test_mixed_format_processing() {
        // Test processing different formats in sequence
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

        // Complex filter: (age > 25 AND active) OR name == "Bob"
        let filter_expr = r#"(age > 25 && active == true) || name == "Bob""#;
        process_with_filter(reader, &mut output, Some(filter_expr)).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        // Should match Alice (30, active) and Bob (25, inactive but name matches) and Charlie (35, active)
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_utility_functions() {
        // Test create_filter
        let filter = create_filter(r#"name == "Alice""#).unwrap();
        match filter {
            FilterExpr::Comparison { field, op, value } => {
                assert_eq!(field.parts, vec!["name"]);
                assert_eq!(op, ComparisonOp::Equal);
                assert_eq!(value, FilterValue::String("Alice".to_string()));
            }
            _ => panic!("Expected comparison"),
        }

        // Test create_template
        let template = create_template("$name is $age").unwrap();
        assert_eq!(template.items.len(), 4);
    }

    #[test]
    fn test_no_filter_passthrough() {
        let input = r#"{"name": "Alice"}
{"name": "Bob"}"#;

        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        // No filter should pass everything through
        process_with_filter(reader, &mut output, None).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();

        assert_eq!(lines.len(), 2);
        // Should be valid JSON output
        let _: Value = serde_json::from_str(lines[0]).unwrap();
        let _: Value = serde_json::from_str(lines[1]).unwrap();
    }
}
