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
//! - **Truthy checks**: `active?`, `user.verified?`, `!disabled?` (checks if fields have truthy values)
//! - **Boolean logic**: `age > 25 && active == true`, `name == "Alice" || name == "Bob"`
//! - **Nested fields**: `user.email == "alice@example.com"`
//! - **Parentheses**: `(age > 25) && (status == "active")`
//! - **Array membership**: `role in ["admin", "moderator"]`, `user.id in allowed_ids`
//!
//! **Note**: Bare field names like `name` are field selectors, not filters.
//! Use explicit comparisons: `name == "Alice"` instead of just `name`.
//! For boolean fields, use the truthy operator: `active?` instead of just `active`.
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
//! - **Indexed fields**: `{${1}, ${2}, ${3}}` (1-based positional access, requires braces)
//! - **Original input**: `{${0}}` (entire original input, requires braces)
//! - **Nested fields**: `{${user.email}}` or `$user.email`
//! - **Literal dollars**: `{Price: $12.50}` (literal $ when not followed by valid variable name)
//!
//! **Variable Mapping Rules:**
//! - `${0}` always refers to the original input text
//! - `${1}`, `${2}`, etc. refer to positional fields (1st, 2nd, etc.)
//! - `$0`, `$1`, `$20` are treated as literal text unless in `${n}` form
//! - Consistent across all data formats (CSV, JSON, text, logfmt, YAML, TOML)
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
//! // Original input variable (${0} always refers to entire input)
//! let dsl = parse_command(r#"{Original: ${0} → Name: ${name}}"#)?;
//! assert!(dsl.template.is_some());
//! if let Some(template) = &dsl.template {
//!     // Example showing ${0} referring to original input
//!     let data = json!({"$0": "Alice,30,Engineer", "name": "Alice"});
//!     let output = template.render(&data);
//!     assert_eq!(output, "Original: Alice,30,Engineer → Name: Alice");
//! }
//!
//! // CSV positional fields (1-based indexing)
//! let dsl = parse_command(r#"{Employee: ${1}, Age: ${2}, Role: ${3}}"#)?;
//! assert!(dsl.template.is_some());
//! if let Some(template) = &dsl.template {
//!     // Example showing how CSV fields map to 1-based indices
//!     let data = json!({"1": "Alice", "2": "30", "3": "Engineer"});
//!     let output = template.render(&data);
//!     assert_eq!(output, "Employee: Alice, Age: 30, Role: Engineer");
//! }
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
//! let dsl = parse_command(r#"age > 25"#)?;
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
//! // Truthy field checks (using ? operator)
//! let dsl = parse_command("active?")?;
//! assert!(dsl.filter.is_some());
//! if let Some(filter) = &dsl.filter {
//!     let data = json!({"name": "Alice", "active": true});
//!     assert!(FilterEngine::evaluate(filter, &data));
//! }
//!
//! // Truthy check with nested fields
//! let dsl = parse_command("user.verified?")?;
//! assert!(dsl.filter.is_some());
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
//! assert!(dsl.filter.is_some());
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
//! // CSV field access patterns (legacy field names still supported)
//! let dsl = parse_command("field_0 == \"Alice\"")?;
//! assert!(dsl.filter.is_some());
//!
//! let dsl = parse_command("field_1 > \"25\"")?;
//! assert!(dsl.filter.is_some());
//!
//! // New 1-based positional access for CSV
//! let dsl = parse_command(r#"{${1}} {${2}} {${3}}"#)?;
//! assert!(dsl.template.is_some());
//!
//! // Text word access patterns (legacy names still supported)
//! let dsl = parse_command("word_0 == \"Alice\"")?;
//! assert!(dsl.filter.is_some());
//!
//! let dsl = parse_command("word_1 > \"25\"")?;
//! assert!(dsl.filter.is_some());
//!
//! // New 1-based positional access for text
//! let dsl = parse_command(r#"{First: ${1}, Second: ${2}}"#)?;
//! assert!(dsl.template.is_some());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Disambiguation Rules
//!
//! The parser follows specific rules to determine how expressions should be interpreted:
//!
//! - **Field selectors**: Bare field names with no operators (`name`, `user.email`)
//! - **Filter expressions**: Explicit comparisons (`age > 25`, `name == "Alice"`)
//! - **Truthy checks**: Field names with `?` suffix (`active?`, `user.verified?`)
//! - **Templates**: Expressions starting with `$` or wrapped in `[]`
//!
//! To avoid ambiguity:
//!
//! - Always use `field?` syntax for truthy checks, not bare field names
//! - Avoid bare field names in boolean expressions (`name && age` is invalid)
//! - Don't mix filter expressions with field selectors
//!
//! ```rust
//! use parsm::parse_command;
//!
//! // These are unambiguous:
//! let dsl1 = parse_command("active?")?; // Filter using truthy check
//! let dsl2 = parse_command("name")?;    // Field selector
//! let dsl3 = parse_command("name == \"Alice\" && age > 25")?; // Filter expression
//!
//! // These would be ambiguous and will be rejected:
//! // parse_command("active && name"); // Ambiguous - both could be field selectors or truthy checks
//! // parse_command("name age");       // Ambiguous - missing operator or invalid syntax
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::error::Error;
use std::io::{BufRead, Write};

// Module declarations
pub mod csv_parser;
pub mod dsl;
pub mod filter;
pub mod format_detector;
pub mod parse;
pub mod parser_registry;

pub use dsl::{parse_command, parse_separate_expressions, ParsedDSL};
pub use filter::{
    ComparisonOp, FieldPath, FilterEngine, FilterExpr, FilterValue, Template, TemplateItem,
};
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
            Ok(_parsed_line) => {
                // Default to returning the original input directly rather than the augmented JSON
                writeln!(writer, "{line}")?;
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

/// Process a single value with filter and template/field selector
/// This is a utility function used by both the main binary and CSV parser to ensure consistent behavior
pub fn process_single_value(
    value: &serde_json::Value,
    dsl: &ParsedDSL,
    writer: &mut impl std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!(
        "process_single_value called with DSL: filter={:?}, template={:?}, field_selector={:?}",
        dsl.filter.is_some(),
        dsl.template.is_some(),
        dsl.field_selector.is_some()
    );

    // Apply filter if present
    let passes_filter = if let Some(ref filter) = dsl.filter {
        let result = FilterEngine::evaluate(filter, value);
        tracing::debug!("Filter evaluation result: {}", result);
        result
    } else {
        true
    };

    if passes_filter {
        // Handle field selection first (takes precedence)
        if let Some(ref field_selector) = dsl.field_selector {
            if let Some(extracted) = field_selector.extract_field(value) {
                tracing::debug!("Field selection extracted: {}", extracted);
                writeln!(writer, "{extracted}")?;
            }
        } else {
            // Handle template or default output
            let output = if let Some(ref template) = dsl.template {
                tracing::debug!("Using template with {} items", template.items.len());
                template.render(value)
            } else {
                tracing::debug!("No template specified, defaulting to ${{0}}");
                // Default to returning ${0} (the original input)
                if let Some(original) = value.get("$0") {
                    if let Some(original_str) = original.as_str() {
                        tracing::debug!("Using original input from $0 (string): {}", original_str);
                        original_str.to_string()
                    } else {
                        tracing::debug!("Using original input from $0 (json): {}", original);
                        serde_json::to_string(original)?
                    }
                } else {
                    tracing::debug!("No $0 field found, using full value: {}", value);
                    serde_json::to_string(value)?
                }
            };
            tracing::debug!("Output: {}", output);
            writeln!(writer, "{output}")?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
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

    #[test]
    fn test_convert_to_json() -> Result<(), Box<dyn Error>> {
        use serde_json::json;

        // Test JSON conversion
        let json_line = parse::ParsedLine::Json(json!({"name": "Alice", "age": 30}));
        let original_json = r#"{"name": "Alice", "age": 30}"#;
        let json_result = convert_to_json(json_line, original_json)?;
        assert_eq!(json_result["name"], "Alice");
        assert_eq!(json_result["age"], 30);
        assert_eq!(json_result["$0"], original_json);

        // Test CSV conversion
        let mut csv_record = csv::StringRecord::new();
        csv_record.push_field("Alice");
        csv_record.push_field("30");
        csv_record.push_field("Engineer");
        let csv_line = parse::ParsedLine::Csv(csv_record);
        let original_csv = "Alice,30,Engineer";
        let csv_result = convert_to_json(csv_line, original_csv)?;
        assert_eq!(csv_result["field_0"], "Alice");
        assert_eq!(csv_result["field_1"], "30");
        assert_eq!(csv_result["field_2"], "Engineer");
        assert_eq!(csv_result["$0"], original_csv);
        assert_eq!(csv_result["1"], "Alice"); // 1-based indexing for templates
        assert_eq!(csv_result["2"], "30");
        assert_eq!(csv_result["3"], "Engineer");

        // Test YAML conversion
        let yaml_data = serde_yaml::from_str::<serde_yaml::Value>("name: Alice\nage: 30").unwrap();
        let yaml_line = parse::ParsedLine::Yaml(yaml_data);
        let original_yaml = "name: Alice\nage: 30";
        let yaml_result = convert_to_json(yaml_line, original_yaml)?;
        assert_eq!(yaml_result["name"], "Alice");
        assert_eq!(yaml_result["age"], 30);
        assert_eq!(yaml_result["$0"], original_yaml);

        // Test Text conversion
        let text_line =
            parse::ParsedLine::Text(vec!["Alice".into(), "30".into(), "Engineer".into()]);
        let original_text = "Alice 30 Engineer";
        let text_result = convert_to_json(text_line, original_text)?;
        assert_eq!(text_result["word_0"], "Alice");
        assert_eq!(text_result["word_1"], "30");
        assert_eq!(text_result["word_2"], "Engineer");
        assert_eq!(text_result["$0"], original_text);
        assert_eq!(text_result["1"], "Alice"); // 1-based indexing for templates
        assert_eq!(text_result["2"], "30");
        assert_eq!(text_result["3"], "Engineer");

        // Test Logfmt conversion
        let mut logfmt_map = serde_json::Map::new();
        logfmt_map.insert("level".into(), json!("error"));
        logfmt_map.insert("msg".into(), json!("timeout"));
        logfmt_map.insert("service".into(), json!("api"));
        let logfmt_line = parse::ParsedLine::Logfmt(serde_json::Value::Object(logfmt_map));
        let original_logfmt = "level=error msg=timeout service=api";
        let logfmt_result = convert_to_json(logfmt_line, original_logfmt)?;
        assert_eq!(logfmt_result["level"], "error");
        assert_eq!(logfmt_result["msg"], "timeout");
        assert_eq!(logfmt_result["service"], "api");
        assert_eq!(logfmt_result["$0"], original_logfmt);

        Ok(())
    }
}
