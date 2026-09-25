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
//! use parsm::{Action, parse_command, process};
//! use std::io::Cursor;
//!
//! // Parse a filter expression
//! let dsl = parse_command(r#"age > 25 {${name} is ${age} years old}"#)?;
//!
//! // Detect the input's format and process each record
//! let input = r#"{"name": "Alice", "age": 30}"#;
//! let mut output = Vec::new();
//! process(Cursor::new(input), None, Action::Evaluate(&dsl), &mut output)?;
//! assert_eq!(String::from_utf8(output)?, "Alice is 30 years old\n");
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
//! use parsm::{Format, Records, detect_format};
//! use std::io::Cursor;
//!
//! // One detector chooses the format from the input
//! assert_eq!(detect_format(r#"{"name": "Alice"}"#), Format::Json);
//! assert_eq!(detect_format("Alice,30,Engineer"), Format::Csv);
//! assert_eq!(detect_format("level=error msg=timeout"), Format::Logfmt);
//!
//! // Records carry their parsed value and their own source text
//! let mut records = Records::open(Cursor::new("level=error msg=timeout"), None)?;
//! let record = records.next().expect("one record")?;
//! assert_eq!(record.source(), "level=error msg=timeout");
//! assert_eq!(record.to_json()["level"], "error");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Architecture
//!
//! The library consists of several key components:
//!
//! - [`detect`]: The one format detector and its precedence table
//! - [`records`]: One record pipeline from input to [`Record`]s, for every format
//! - [`pipeline`]: Records to output, converted or filtered and rendered
//! - [`filter`]: Boolean expression evaluation engine
//! - [`dsl`]: Domain-specific language parser using Pest
//!
//! ## Error Handling
//!
//! - **First record errors**: Fatal when the format is forced; a guess falls
//!   through to the next format
//! - **Later record errors**: Warnings; the record is skipped
//! - **Missing fields**: The record prints nothing
//!
//! ## Performance
//!
//! - **Streaming**: JSON, logfmt, CSV and text records are written as they complete
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
//! use parsm::{Action, parse_command, process};
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
//! let dsl = parse_command("name")?;
//! let input = r#"{"name": "Alice", "age": 30}"#;
//! let mut output = Vec::new();
//! process(Cursor::new(input), None, Action::Evaluate(&dsl), &mut output)?;
//! assert_eq!(String::from_utf8(output)?, "Alice\n");
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
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ```rust
//! use parsm::parse_command;
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
//! use parsm::parse_command;
//!
//! // CSV field access patterns (legacy field names still supported)
//! let dsl = parse_command("field_0 == \"Alice\"")?;
//! assert!(dsl.filter.is_some());
//!
//! let dsl = parse_command("field_1 > \"25\"")?;
//! assert!(dsl.filter.is_some());
//!
//! // New 1-based positional access for CSV
//! let dsl = parse_command(r#"{${1}, ${2}, ${3}}"#)?;
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

// Module declarations
pub mod csv_parser;
pub mod detect;
mod documents;
pub mod dsl;
pub mod filter;
pub mod format_detector;
mod json_stream;
mod lines;
pub mod parse;
pub mod parser_registry;
pub mod pipeline;
pub mod record;
pub mod records;

pub use detect::{Format, detect_format};
pub use dsl::{ParsedDSL, parse_command, parse_separate_expressions};
pub use filter::{
    ComparisonOp, FieldPath, FilterEngine, FilterExpr, FilterValue, Template, TemplateItem,
};
pub use format_detector::{DetectedFormat, FormatDetector};
pub use parser_registry::{DocumentParser, ParserRegistry};
pub use pipeline::{Action, process};
pub use record::Record;
pub use records::{RecordError, Records};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::error::Error;
    use std::io::Cursor;

    fn convert(input: &str) -> Result<String, Box<dyn Error>> {
        let mut output = Vec::new();
        process(Cursor::new(input), None, Action::Convert, &mut output)?;
        Ok(String::from_utf8(output)?)
    }

    #[test]
    fn test_end_to_end_json_processing() -> Result<(), Box<dyn Error>> {
        let result = convert(r#"{"name": "Alice", "age": 30}"#)?;
        assert_eq!(result, "{\"name\":\"Alice\",\"age\":30}\n");
        Ok(())
    }

    #[test]
    fn test_end_to_end_csv_processing() -> Result<(), Box<dyn Error>> {
        let result = convert("Alice,30,Engineer")?;
        assert_eq!(result, "[\"Alice\",\"30\",\"Engineer\"]\n");
        Ok(())
    }

    #[test]
    fn test_end_to_end_logfmt_processing() -> Result<(), Box<dyn Error>> {
        let result = convert("level=error msg=timeout service=api")?;
        assert_eq!(
            result,
            "{\"level\":\"error\",\"msg\":\"timeout\",\"service\":\"api\"}\n"
        );
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
        assert_eq!(convert(r#"{"name": "Alice"}"#)?, "{\"name\":\"Alice\"}\n");
        assert_eq!(convert("Alice,30")?, "[\"Alice\",\"30\"]\n");
        assert_eq!(convert("name: Alice")?, "{\"name\":\"Alice\"}\n");
        Ok(())
    }

    #[test]
    fn test_filter_writes_matching_source() -> Result<(), Box<dyn Error>> {
        let dsl = parse_command("age > 25")?;
        let input = "{\"name\": \"Alice\", \"age\": 30}\n{\"name\": \"Bob\", \"age\": 20}";
        let mut output = Vec::new();
        process(
            Cursor::new(input),
            None,
            Action::Evaluate(&dsl),
            &mut output,
        )?;
        assert_eq!(
            String::from_utf8(output)?,
            "{\"name\": \"Alice\", \"age\": 30}\n"
        );
        Ok(())
    }

    #[test]
    fn test_utility_functions() -> Result<(), Box<dyn Error>> {
        let dsl = parse_command("name")?;
        assert!(dsl.field_selector.is_some());

        let dsl = parse_command(r#"age > 25"#)?;
        assert!(dsl.filter.is_some());

        let dsl = parse_command("$name")?;
        assert!(dsl.template.is_some());

        Ok(())
    }
}
