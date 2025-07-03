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
//! - **Boolean logic**: `age > 25 && active == true`, `name == "Alice" || name == "Bob"`
//! - **Nested fields**: `user.email == "alice@example.com"`
//! - **Parentheses**: `(age > 25) && (status == "active")`
//!
//! **Note**: Bare field names like `name` are field selectors, not filters.
//! Use explicit comparisons: `name == "Alice"` instead of just `name`.
//!
//! ## Template Syntax
//!
//! Templates use `${variable}` for field substitution. There are several ways to create templates:
//!
//! - **Braced templates**: `{${name} is ${age}}` (explicit field variables)
//! - **Simple variables**: `$name` (becomes a field template)
//! - **Mixed templates**: `{Hello ${name}!}` (mix literals and variables)
//! - **Interpolated text**: `Hello $name` (variables in plain text)
//! - **Literal templates**: `{name}` (literal text, not field substitution)
//! - **Indexed fields**: `{${1}, ${2}, ${3}}` (1-based, requires braces for numbers)
//! - **Original input**: `{${0}}` (entire original input, requires braces)
//! - **Nested fields**: `{${user.email}}` or `$user.email`
//! - **Literal dollars**: `{Price: $12.50}` (literal $ when not followed by valid variable name)
//!
//! ## Field Selection
//!
//! Field selectors extract specific fields from data. Both quoted and unquoted syntax work:
//!
//! - **Simple fields**: `name`, `age`, `status`
//! - **Nested fields**: `user.email`, `config.database.host`
//! - **Quoted fields**: `"field name"`, `"user.email"` (for names with spaces or special chars)
//!
//! ```rust
//! use parsm::parse_command;
//!
//! // Extract fields using simple syntax
//! let dsl = parse_command("user.email")?;
//! assert!(dsl.field_selector.is_some());
//!
//! // Or quoted syntax for complex names
//! let dsl = parse_command(r#""field with spaces""#)?;
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
//!
//! ## Comprehensive Examples from README
//!
//! All examples from the README are tested here to ensure documentation accuracy.
//!
//! ### Field Extraction Examples
//!
//! ```rust
//! use parsm::{parse_command, process_stream};
//! use serde_json::json;
//! use std::io::Cursor;
//!
//! // Simple field extraction
//! let dsl = parse_command("name")?;
//! assert!(dsl.field_selector.is_some());
//!
//! // Nested field access
//! let dsl = parse_command("user.email")?;
//! assert!(dsl.field_selector.is_some());
//!
//! // Array element access
//! let dsl = parse_command("items.0")?;
//! assert!(dsl.field_selector.is_some());
//!
//! // Process real data with field extraction
//! let input = r#"{"name": "Alice", "age": 30}"#;
//! let mut output = Vec::new();
//! process_stream(Cursor::new(input), &mut output)?;
//! let result = String::from_utf8(output)?;
//! assert!(result.contains("Alice") || result.contains("30"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Template Examples
//!
//! ```rust
//! use parsm::parse_command;
//! use serde_json::json;
//!
//! // Variable template with braces
//! let dsl = parse_command(r#"{${name} is ${age} years old}"#)?;
//! assert!(dsl.template.is_some());
//! if let Some(template) = &dsl.template {
//!     let data = json!({"name": "Alice", "age": 30});
//!     let output = template.render(&data);
//!     assert_eq!(output, "Alice is 30 years old");
//! }
//!
//! // Simple variable shorthand
//! let dsl = parse_command("$name")?;
//! assert!(dsl.template.is_some());
//! if let Some(template) = &dsl.template {
//!     let data = json!({"name": "Alice"});
//!     let output = template.render(&data);
//!     assert_eq!(output, "Alice");
//! }
//!
//! // Literal template (no variables)
//! let dsl = parse_command("{name}")?;
//! assert!(dsl.template.is_some());
//! if let Some(template) = &dsl.template {
//!     let data = json!({"name": "Alice"});
//!     let output = template.render(&data);
//!     assert_eq!(output, "name");
//! }
//!
//! // Original input variable
//! let dsl = parse_command(r#"{Original: ${0} → Name: ${name}}"#)?;
//! assert!(dsl.template.is_some());
//!
//! // CSV positional fields
//! let dsl = parse_command(r#"{Employee: ${1}, Age: ${2}, Role: ${3}}"#)?;
//! assert!(dsl.template.is_some());
//!
//! // Nested JSON fields
//! let dsl = parse_command(r#"{User: ${user.name}, Email: ${user.email}}"#)?;
//! assert!(dsl.template.is_some());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Filter Examples
//!
//! ```rust
//! use parsm::{parse_command, FilterEngine};
//! use serde_json::json;
//!
//! // Basic filtering
//! let dsl = parse_command("age > 25")?;
//! assert!(dsl.filter.is_some());
//! if let Some(filter) = &dsl.filter {
//!     let data = json!({"name": "Alice", "age": 30});
//!     assert!(FilterEngine::evaluate(filter, &data));
//! }
//!
//! // String equality
//! let dsl = parse_command(r#"name == "Alice""#)?;
//! assert!(dsl.filter.is_some());
//! if let Some(filter) = &dsl.filter {
//!     let data = json!({"name": "Alice", "age": 30});
//!     assert!(FilterEngine::evaluate(filter, &data));
//! }
//!
//! // Boolean comparison
//! let dsl = parse_command("user.active == true")?;
//! assert!(dsl.filter.is_some());
//!
//! // Negation
//! let dsl = parse_command(r#"!(status == "disabled")"#)?;
//! assert!(dsl.filter.is_some());
//!
//! // Boolean logic
//! let dsl = parse_command(r#"name == "Alice" && age > 25"#)?;
//! assert!(dsl.filter.is_some());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Combined Filter and Template Examples
//!
//! ```rust
//! use parsm::{parse_command, FilterEngine};
//! use serde_json::json;
//!
//! // Filter with template output
//! let dsl = parse_command(r#"age > 25 {${name} is ${age} years old}"#)?;
//! assert!(dsl.filter.is_some());
//! assert!(dsl.template.is_some());
//!
//! let data = json!({"name": "Alice", "age": 30});
//! if let (Some(filter), Some(template)) = (&dsl.filter, &dsl.template) {
//!     if FilterEngine::evaluate(filter, &data) {
//!         let output = template.render(&data);
//!         assert_eq!(output, "Alice is 30 years old");
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Field Selection Examples
//!
//! ```rust
//! use parsm::parse_command;
//!
//! // Simple field extraction
//! let dsl = parse_command("name")?;
//! assert!(dsl.field_selector.is_some());
//! assert!(dsl.filter.is_none());
//! assert!(dsl.template.is_none());
//!
//! // Nested field access
//! let dsl = parse_command("user.email")?;
//! assert!(dsl.field_selector.is_some());
//!
//! // Array element access
//! let dsl = parse_command("items.0")?;
//! assert!(dsl.field_selector.is_some());
//!
//! // Quoted field names
//! let dsl = parse_command(r#""field name""#)?;
//! assert!(dsl.field_selector.is_some());
//!
//! let dsl = parse_command("'special-field'")?;
//! assert!(dsl.field_selector.is_some());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### String Operations Examples
//!
//! ```rust
//! use parsm::{parse_command, FilterEngine};
//! use serde_json::json;
//!
//! // Contains substring
//! let dsl = parse_command(r#"email ~ "@company.com""#)?;
//! assert!(dsl.filter.is_some());
//! if let Some(filter) = &dsl.filter {
//!     let data = json!({"email": "alice@company.com"});
//!     assert!(FilterEngine::evaluate(filter, &data));
//! }
//!
//! // Starts with prefix
//! let dsl = parse_command(r#"name ^= "A""#)?;
//! assert!(dsl.filter.is_some());
//! if let Some(filter) = &dsl.filter {
//!     let data = json!({"name": "Alice"});
//!     assert!(FilterEngine::evaluate(filter, &data));
//! }
//!
//! // Ends with suffix
//! let dsl = parse_command(r#"file $= ".log""#)?;
//! assert!(dsl.filter.is_some());
//! if let Some(filter) = &dsl.filter {
//!     let data = json!({"file": "app.log"});
//!     assert!(FilterEngine::evaluate(filter, &data));
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Comparison Operators Examples
//!
//! ```rust
//! use parsm::parse_command;
//!
//! // All comparison operators
//! let operators = vec![
//!     r#"name == "Alice""#,
//!     r#"status != "inactive""#,
//!     "age < 30",
//!     "score <= 95",
//!     "age > 18",
//!     "score >= 90",
//! ];
//!
//! for op in operators {
//!     let dsl = parse_command(op)?;
//!     assert!(dsl.filter.is_some());
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Boolean Logic Examples
//!
//! ```rust
//! use parsm::parse_command;
//!
//! // Logical AND
//! let dsl = parse_command("age > 18 && active == true")?;
//! assert!(dsl.filter.is_some());
//!
//! // Logical OR
//! let dsl = parse_command(r#"role == "admin" || role == "user""#)?;
//! assert!(dsl.filter.is_some());
//!
//! // Logical NOT
//! let dsl = parse_command(r#"!(status == "disabled")"#)?;
//! assert!(dsl.filter.is_some());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Advanced Boolean Logic Examples
//!
//! ```rust
//! use parsm::parse_command;
//!
//! // Multiple conditions with parentheses
//! let dsl = parse_command(r#"name == "Alice" && (age > 25 || active == true)"#)?;
//! assert!(dsl.filter.is_some());
//!
//! // Complex negation
//! let dsl = parse_command(r#"!(status == "disabled" || role == "guest")"#)?;
//! assert!(dsl.filter is_some());
//!
//! // String operations with boolean logic
//! let dsl = parse_command(r#"email ~ "@company.com" && name ^= "A""#)?;
//! assert!(dsl.filter.is_some());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Format-Specific Examples
//!
//! ```rust
//! use parsm::{parse_command, StreamingParser};
//!
//! // CSV field access patterns
//! let dsl = parse_command("field_0 == \"Alice\"")?;
//! assert!(dsl.filter.is_some());
//!
//! let dsl = parse_command("field_1 > \"25\"")?;
//! assert!(dsl.filter.is_some());
//!
//! // Text word access patterns
//! let dsl = parse_command("word_0 == \"Alice\"")?;
//! assert!(dsl.filter is_some());
//!
//! let dsl = parse_command("word_1 > \"25\"")?;
//! assert!(dsl.filter is_some());
//!
//! // Verify different formats can be parsed
//! let mut parser = StreamingParser::new();
//!
//! // JSON
//! let _json_result = parser.parse_line(r#"{"name": "Alice", "age": 30}"#)?;
//!
//! // Reset parser for different format
//! let mut parser = StreamingParser::new();
//! // CSV  
//! let _csv_result = parser.parse_line("Alice,30,Engineer")?;
//!
//! // Reset parser for different format
//! let mut parser = StreamingParser::new();
//! // YAML
//! let _yaml_result = parser.parse_line("name: Alice")?;
//!
//! // Reset parser for different format  
//! let mut parser = StreamingParser::new();
//! // TOML
//! let _toml_result = parser.parse_line(r#"name = "Alice""#)?;
//!
//! // Reset parser for different format
//! let mut parser = StreamingParser::new();
//! // Logfmt
//! let _logfmt_result = parser.parse_line("level=error msg=timeout")?;
//!
//! // Reset parser for different format
//! let mut parser = StreamingParser::new();
//! // Plain text
//! let _text_result = parser.parse_line("Alice 30 Engineer")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Real-World Processing Examples
//!
//! ```rust
//! use parsm::{parse_command, process_stream};
//! use std::io::Cursor;
//!
//! // JSON processing with field extraction
//! let input = r#"{"name": "Alice", "age": 30}"#;
//! let mut output = Vec::new();
//! process_stream(Cursor::new(input), &mut output)?;
//! let result = String::from_utf8(output)?;
//! // Should contain the original JSON data
//! assert!(result.contains("Alice"));
//!
//! // CSV processing
//! let input = "Alice,30,Engineer";
//! let mut output = Vec::new();
//! process_stream(Cursor::new(input), &mut output)?;
//! let result = String::from_utf8(output)?;
//! // Should contain converted JSON format
//! assert!(result.contains("Alice"));
//!
//! // YAML processing
//! let input = "name: Alice\nage: 30";
//! let mut output = Vec::new();
//! process_stream(Cursor::new(input), &mut output)?;
//! let result = String::from_utf8(output)?;
//! // Should contain converted JSON format
//! assert!(result.contains("Alice"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::error::Error;
use std::io::{BufRead, Write};

pub mod csv_parser;
pub mod dsl;
pub mod filter;
pub mod format_detector;
pub mod operators;
pub mod parse;
pub mod parser_registry;

pub use dsl::{parse_command, ParsedDSL};
pub use filter::{FieldPath, FilterEngine, FilterExpr, Template};
pub use format_detector::{DetectedFormat, FormatDetector};
pub use parse::{ParsedLine, StreamingParser};
pub use parser_registry::{DocumentParser, ParserRegistry};

/// Process a stream of input data with optional DSL filter and template
pub fn process_stream<R: BufRead, W: Write>(
    reader: R,
    writer: &mut W,
) -> Result<(), Box<dyn Error>> {
    let mut parser = StreamingParser::new();
    let mut line_count = 0;

    for line_result in reader.lines() {
        line_count += 1;
        let line = line_result?;

        if line.trim().is_empty() {
            continue;
        }

        match parser.parse_line(&line) {
            Ok(parsed_line) => {
                let json_value = convert_to_json(parsed_line, &line)?;
                writeln!(writer, "{}", serde_json::to_string(&json_value)?)?;
            }
            Err(e) => {
                if line_count == 1 {
                    return Err(Box::new(e));
                } else {
                    eprintln!("Warning: Failed to parse line {line_count}: {e}");
                }
            }
        }
    }

    Ok(())
}

/// Process a stream with a specific DSL command
pub fn process_stream_with_dsl<R: BufRead, W: Write>(
    reader: R,
    writer: &mut W,
    dsl: &ParsedDSL,
) -> Result<(), Box<dyn Error>> {
    let mut parser = StreamingParser::new();
    let mut line_count = 0;

    for line_result in reader.lines() {
        line_count += 1;
        let line = line_result?;

        if line.trim().is_empty() {
            continue;
        }

        match parser.parse_line(&line) {
            Ok(parsed_line) => {
                let json_value = convert_to_json(parsed_line, &line)?;

                // Handle field selection
                if let Some(ref field_selector) = dsl.field_selector {
                    if let Some(field_value) = field_selector.extract_field(&json_value) {
                        writeln!(writer, "{field_value}")?;
                    }
                    continue;
                }

                // Apply filter if present
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

                    writeln!(writer, "{output}")?;
                }
            }
            Err(e) => {
                if line_count == 1 {
                    return Err(Box::new(e));
                } else {
                    eprintln!("Warning: Failed to parse line {line_count}: {e}");
                }
            }
        }
    }

    Ok(())
}

fn convert_to_json(
    parsed_line: parse::ParsedLine,
    original_input: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use serde_json::Value;

    let mut json_value = match parsed_line {
        parse::ParsedLine::Json(mut val) => {
            if let Value::Object(ref mut obj) = val {
                obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            }
            val
        }
        parse::ParsedLine::Csv(record) => {
            let mut obj = serde_json::Map::new();
            obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            for (i, field) in record.iter().enumerate() {
                obj.insert(format!("field_{i}"), Value::String(field.to_string()));
            }
            let values: Vec<Value> = record
                .iter()
                .map(|field| Value::String(field.to_string()))
                .collect();
            obj.insert("_array".to_string(), Value::Array(values));
            Value::Object(obj)
        }
        parse::ParsedLine::Toml(val) => {
            let mut json_val = serde_json::to_value(val)?;
            if let Value::Object(ref mut obj) = json_val {
                obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            }
            json_val
        }
        parse::ParsedLine::Yaml(val) => {
            let mut json_val = serde_json::to_value(val)?;
            if let Value::Object(ref mut obj) = json_val {
                obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            }
            json_val
        }
        parse::ParsedLine::Logfmt(mut val) => {
            if let Value::Object(ref mut obj) = val {
                obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            }
            val
        }
        parse::ParsedLine::Text(words) => {
            let mut obj = serde_json::Map::new();
            obj.insert("$0".to_string(), Value::String(original_input.to_string()));
            for (i, word) in words.iter().enumerate() {
                let word = Value::String(word.clone());
                obj.insert(format!("word_{i}"), word);
            }
            let values: Vec<Value> = words.into_iter().map(Value::String).collect();
            obj.insert("_array".to_string(), Value::Array(values));
            Value::Object(obj)
        }
    };

    // Add indexed fields (1-based) for templates
    if let Value::Object(ref mut obj) = json_value {
        if let Some(Value::Array(ref arr)) = obj.get("_array").cloned() {
            for (i, value) in arr.iter().enumerate() {
                obj.insert((i + 1).to_string(), value.clone());
            }
        }
    }

    Ok(json_value)
}

/// Parse filter and template expressions separately (legacy compatibility function)
pub fn parse_separate_expressions(
    filter: Option<&str>,
    template: Option<&str>,
) -> Result<ParsedDSL, Box<dyn Error>> {
    let mut dsl = ParsedDSL::new();

    if let Some(filter_str) = filter {
        if !filter_str.trim().is_empty() {
            let filter_dsl = parse_command(filter_str)?;
            dsl.filter = filter_dsl.filter;
        }
    }

    if let Some(template_str) = template {
        if !template_str.trim().is_empty() {
            let template_dsl = parse_command(template_str)?;
            dsl.template = template_dsl.template;
        }
    }

    Ok(dsl)
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::io::BufReader;
    use std::io::Cursor;

    #[test]
    fn test_end_to_end_json_processing() -> Result<(), Box<dyn Error>> {
        let input = r#"{"name": "Alice", "age": 30}"#;
        let mut output = Vec::new();
        process_stream(BufReader::new(Cursor::new(input)), &mut output)?;

        let result = String::from_utf8(output)?;
        assert!(result.contains("Alice"));
        assert!(result.contains("30"));
        Ok(())
    }

    #[test]
    fn test_end_to_end_csv_processing() -> Result<(), Box<dyn Error>> {
        let input = "Alice,30,Engineer";
        let mut output = Vec::new();
        process_stream(BufReader::new(Cursor::new(input)), &mut output)?;

        let result = String::from_utf8(output)?;
        assert!(result.contains("Alice"));
        assert!(result.contains("30"));
        assert!(result.contains("Engineer"));
        Ok(())
    }

    #[test]
    fn test_end_to_end_logfmt_processing() -> Result<(), Box<dyn Error>> {
        let input = "level=error msg=timeout service=api";
        let mut output = Vec::new();
        process_stream(BufReader::new(Cursor::new(input)), &mut output)?;

        let result = String::from_utf8(output)?;
        assert!(result.contains("error"));
        assert!(result.contains("timeout"));
        assert!(result.contains("api"));
        Ok(())
    }

    #[test]
    fn test_complex_filter_expressions() -> Result<(), Box<dyn Error>> {
        let dsl = parse_command(r#"age > 25 && name == "Alice""#)?;
        assert!(dsl.filter.is_some());
        Ok(())
    }

    #[test]
    fn test_mixed_format_processing() -> Result<(), Box<dyn Error>> {
        // Test that different parsers can handle their respective formats
        let json_input = r#"{"name": "Alice"}"#;
        let csv_input = "Alice,30";
        let yaml_input = "name: Alice";

        let mut json_output = Vec::new();
        let mut csv_output = Vec::new();
        let mut yaml_output = Vec::new();

        process_stream(BufReader::new(Cursor::new(json_input)), &mut json_output)?;
        process_stream(BufReader::new(Cursor::new(csv_input)), &mut csv_output)?;
        process_stream(BufReader::new(Cursor::new(yaml_input)), &mut yaml_output)?;

        assert!(!json_output.is_empty());
        assert!(!csv_output.is_empty());
        assert!(!yaml_output.is_empty());
        Ok(())
    }

    #[test]
    fn test_no_filter_passthrough() -> Result<(), Box<dyn Error>> {
        let input = r#"{"name": "Alice", "age": 30}"#;
        let mut output = Vec::new();
        process_stream(BufReader::new(Cursor::new(input)), &mut output)?;

        let result = String::from_utf8(output)?;
        // Should pass through and convert to JSON
        assert!(result.contains("Alice"));
        Ok(())
    }

    #[test]
    fn test_utility_functions() -> Result<(), Box<dyn Error>> {
        // Test that basic utility functions work
        let dsl = parse_command("name")?;
        assert!(dsl.field_selector.is_some());

        let dsl = parse_command(r#"age > 25"#)?;
        assert!(dsl.filter.is_some());

        let dsl = parse_command("$name")?;
        assert!(dsl.template.is_some());

        Ok(())
    }
}
