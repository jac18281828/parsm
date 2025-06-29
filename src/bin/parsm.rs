use clap::{Arg, Command};
use std::io;

use parsm::{
    parse_command, parse_separate_expressions, process_stream, FilterEngine, ParsedDSL, ParsedLine,
};

/// Main entry point for the parsm command-line tool.
///
/// Parsm is a multi-format data processor that understands structured text better than sed or awk.
/// It can parse JSON, CSV, TOML, YAML, logfmt, and plain text, applying filters and templates
/// to transform and extract data.
fn main() {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Understands structured text better than sed or awk")
        .arg(
            Arg::new("filter")
                .help("Filter expression (optional)")
                .value_name("FILTER")
                .index(1),
        )
        .arg(
            Arg::new("template")
                .help("Template expression for output formatting (optional)")
                .value_name("TEMPLATE")
                .index(2),
        )
        .arg(
            Arg::new("help-examples")
                .long("examples")
                .help("Show usage examples")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    if matches.get_flag("help-examples") {
        print_usage_examples();
        return;
    }

    let filter_expr = matches.get_one::<String>("filter");
    let template_expr = matches.get_one::<String>("template");

    match (filter_expr, template_expr) {
        (Some(filter), Some(template)) if !filter.trim().is_empty() => {
            let parsed_dsl = match parse_separate_expressions(Some(filter), Some(template)) {
                Ok(dsl) => dsl,
                Err(e) => {
                    eprintln!("Error parsing filter and template expression: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = process_stream_with_filter(parsed_dsl) {
                eprintln!("Error processing stream: {}", e);
                std::process::exit(1);
            }
        }
        (Some(_), Some(template)) => {
            let parsed_dsl = match parse_separate_expressions(None, Some(template)) {
                Ok(dsl) => dsl,
                Err(e) => {
                    eprintln!("Error parsing template expression: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = process_stream_with_filter(parsed_dsl) {
                eprintln!("Error processing stream: {}", e);
                std::process::exit(1);
            }
        }
        (Some(filter), None) => {
            let parsed_dsl = match parse_command(filter) {
                Ok(dsl) => dsl,
                Err(e) => {
                    eprintln!("Error parsing expression: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = process_stream_with_filter(parsed_dsl) {
                eprintln!("Error processing stream: {}", e);
                std::process::exit(1);
            }
        }
        (None, Some(template)) => {
            let parsed_dsl = match parse_separate_expressions(None, Some(template)) {
                Ok(dsl) => dsl,
                Err(e) => {
                    eprintln!("Error parsing template expression: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = process_stream_with_filter(parsed_dsl) {
                eprintln!("Error processing stream: {}", e);
                std::process::exit(1);
            }
        }
        (None, None) => {
            let stdin = io::stdin();
            let mut stdout = io::stdout();

            if let Err(e) = process_stream(stdin.lock(), &mut stdout) {
                eprintln!("Error processing stream: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Process input stream with the parsed DSL (filters, templates, field selectors).
///
/// This function handles different processing modes:
/// - Field selection: Extract specific fields from JSON objects/arrays
/// - Filtering: Apply boolean expressions to filter input lines
/// - Templates: Format output using template expressions
///
/// # Arguments
/// * `dsl` - Parsed DSL containing optional filter, template, and field selector
///
/// # Returns
/// * `Ok(())` on successful processing
/// * `Err(Box<dyn std::error::Error>)` on processing errors
fn process_stream_with_filter(dsl: ParsedDSL) -> Result<(), Box<dyn std::error::Error>> {
    use parsm::StreamingParser;
    use std::io::{BufRead, Read, Write};
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    // Only read entire input for field selectors or when necessary for document parsing
    if let Some(ref field_selector) = dsl.field_selector {
        // Field selectors need the entire input to handle JSON arrays
        let mut input = String::new();
        stdin.lock().read_to_string(&mut input)?;

        // Try JSON array parsing first
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&input) {
            match &json_value {
                serde_json::Value::Array(arr) => {
                    for item in arr {
                        if let Some(extracted) = field_selector.extract_field(item) {
                            writeln!(writer, "{}", extracted)?;
                        }
                    }
                    return Ok(());
                }
                _ => {
                    if let Some(extracted) = field_selector.extract_field(&json_value) {
                        writeln!(writer, "{}", extracted)?;
                    }
                    return Ok(());
                }
            }
        }

        // Try other document formats
        if try_parse_as_toml(&input, &dsl, &mut writer)?.is_some() {
            return Ok(());
        }

        if try_parse_as_yaml(&input, &dsl, &mut writer)?.is_some() {
            return Ok(());
        }

        // Fall back to line-by-line processing for field selectors
        let lines = input.lines();
        let mut parser = StreamingParser::new();
        let mut line_count = 0;

        for line in lines {
            line_count += 1;

            if line.trim().is_empty() {
                continue;
            }

            match parser.parse_line(line) {
                Ok(parsed_line) => {
                    let json_value = convert_parsed_line_to_json(parsed_line, line)?;
                    if let Some(extracted) = field_selector.extract_field(&json_value) {
                        writeln!(writer, "{}", extracted)?;
                    } else {
                        writeln!(writer)?;
                        eprintln!(
                            "Warning: Field '{}' not found in line {}",
                            field_selector.parts.join("."),
                            line_count
                        );
                    }
                }
                Err(e) => {
                    if line_count == 1 {
                        return Err(Box::new(e));
                    } else {
                        eprintln!("Warning: Failed to parse line {}: {}", line_count, e);
                        eprintln!("Line content: {}", line);
                    }
                }
            }
        }
    } else {
        // For filters and templates, use true streaming (line-by-line processing)
        let reader = stdin.lock();
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
                    let json_value = convert_parsed_line_to_json(parsed_line, &line)?;

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
                        eprintln!("Line content: {}", line);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Convert a parsed line to a JSON value.
///
/// This function takes a `ParsedLine` from the parser and converts it to a `serde_json::Value`
/// for consistent processing. It also adds the original input as a special `$$` field.
///
/// # Arguments
/// * `parsed_line` - The parsed line data structure
/// * `original_input` - The original input string that was parsed
///
/// # Returns
/// * `Ok(serde_json::Value)` - The converted JSON value
/// * `Err(Box<dyn std::error::Error>)` - Conversion error
fn convert_parsed_line_to_json(
    parsed_line: ParsedLine,
    original_input: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use serde_json::Value;

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

/// Print comprehensive usage examples and help documentation.
///
/// This function displays detailed examples of how to use parsm for various data processing
/// tasks including filtering, field selection, template formatting, and format conversion.
fn print_usage_examples() {
    println!("parsm - Multi-format data processor");
    println!();
    println!("EXAMPLES:");
    println!();
    println!("  # Filter JSON by field value:");
    println!(r#"  echo '{{"name": "Alice", "age": 30}}' | parsm 'name == "Alice"'"#);
    println!();
    println!("  # Field selection:");
    println!(r#"  echo '{{"name": "Alice", "age": 30}}' | parsm 'name'"#);
    println!();
    println!("  # Filter and format output (combined):");
    println!(
        r#"  echo '{{"name": "Alice", "age": 30}}' | parsm 'age > 25 {{${{name}} is ${{age}} years old}}'"#
    );
    println!();
    println!("  # Filter and format output (separate arguments):");
    println!(
        r#"  echo '{{"name": "Alice", "age": 30}}' | parsm 'age > 25' '${{name}} is ${{age}} years old'"#
    );
    println!();
    println!("  # Simple template variables:");
    println!(r#"  echo '{{"name": "Alice", "age": 30}}' | parsm '$name is $age years old'"#);
    println!();
    println!("  # Include original input with $0:");
    println!(r#"  echo 'Alice,30' | parsm '${{0}} → ${{1}} is ${{2}}'"#);
    println!();
    println!("  # Filter CSV data (fields accessible as field_0, field_1, etc.):");
    println!(
        r#"  echo 'Alice,30,Engineer' | parsm 'field_1 > "25" {{${{field_0}} - ${{field_2}}}}'"#
    );
    println!();
    println!("  # Filter logfmt logs:");
    println!(
        r#"  echo 'level=error msg="DB error" service=api' | parsm 'level == "error" {{[${{level}}] ${{msg}}}}'"#
    );
    println!();
    println!("  # Complex conditions:");
    println!(r#"  parsm 'name == "Alice" && age > 25 {{${{name}}: active}}'"#);
    println!();
    println!("  # Just convert formats (no filter):");
    println!("  echo 'name: Alice' | parsm  # YAML to JSON");
    println!();
    println!("OPERATORS:");
    println!("  ==, !=, <, <=, >, >=        # Comparison");
    println!("  contains, startswith, endswith  # String operations");
    println!("  &&, ||, !                   # Boolean logic");
    println!();
    println!("FIELD ACCESS:");
    println!("  name                        # Field selection (bare identifier)");
    println!("  \"name\"                      # Field selection (quoted)");
    println!("  user.email                  # Nested field");
    println!("  field_0, field_1            # CSV columns");
    println!("  word_0, word_1              # Text words");
    println!();
    println!("TEMPLATE VARIABLES:");
    println!("  ${{0}}                        # Entire original input");
    println!("  ${{1}}, ${{2}}, ${{3}}              # Indexed fields (1-based, requires braces)");
    println!("  $name, ${{user.email}}        # Named fields ($simple or ${{complex}})");
    println!("  $100                        # Literal dollar amounts (invalid variable names)");
    println!();
}

/// Determines if the input should be parsed as a complete document
/// based on content analysis and first character(s).
fn should_parse_entire_document(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return false;
    }

    let first_char = trimmed.chars().next().unwrap();

    match first_char {
        // JSON objects and arrays - parse as complete document
        '{' | '[' => true,

        // TOML typically starts with [section] or key = value
        // (Note: '[' is already handled above for JSON arrays)

        // Check for TOML key = value pattern at start
        _ if is_likely_toml(trimmed) => true,

        // YAML documents - check for YAML indicators
        _ if is_likely_yaml(trimmed) => true,

        // Quote-started content might be a single JSON string, but more likely line-by-line
        '"' => false,

        // Everything else (plain text, CSV, logfmt) - process line by line
        _ => false,
    }
}

/// Check if content looks like TOML format
fn is_likely_toml(input: &str) -> bool {
    let lines: Vec<&str> = input.lines().take(10).collect(); // Check first 10 lines

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for key = value pattern typical of TOML
        if trimmed.contains(" = ") && !trimmed.starts_with('"') {
            return true;
        }

        // Look for TOML section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            return true;
        }
    }

    false
}

/// Check if content looks like YAML format
fn is_likely_yaml(input: &str) -> bool {
    let lines: Vec<&str> = input.lines().take(10).collect(); // Check first 10 lines

    // YAML document start indicator
    if input.trim_start().starts_with("---") {
        return true;
    }

    let mut has_yaml_structure = false;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for YAML key: value pattern (with colon and space)
        if trimmed.contains(": ") && !trimmed.starts_with('"') {
            has_yaml_structure = true;
        }

        // Look for YAML list items
        if trimmed.starts_with("- ") {
            has_yaml_structure = true;
        }

        // Look for indented structure (common in YAML)
        if line.starts_with("  ") && (line.contains(": ") || line.trim().starts_with("- ")) {
            return true; // Strong indicator of YAML structure
        }
    }

    has_yaml_structure
}

/// Try to parse input as JSON and process it
fn try_parse_as_json(
    input: &str,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<Option<()>, Box<dyn std::error::Error>> {
    let trimmed = input.trim();

    // Check if this looks like a large JSON array that should be streamed
    if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 1024 * 1024 {
        // For large JSON arrays (>1MB), use streaming parser to avoid loading entire array into memory
        if let Ok(()) = parse_json_array_streaming(trimmed, dsl, writer) {
            return Ok(Some(()));
        }
    }

    // For single JSON objects or smaller arrays, use the standard parser
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(input) {
        process_structured_value(json_value, input, dsl, writer)?;
        Ok(Some(()))
    } else {
        Ok(None)
    }
}

/// Try to parse input as TOML and process it
fn try_parse_as_toml(
    input: &str,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<Option<()>, Box<dyn std::error::Error>> {
    // Only try TOML parsing if the input actually looks like TOML
    if !is_likely_toml(input) {
        return Ok(None);
    }

    if let Ok(toml_value) = toml::from_str::<toml::Value>(input) {
        let json_value = serde_json::to_value(toml_value)?;
        process_structured_value(json_value, input, dsl, writer)?;
        Ok(Some(()))
    } else {
        Ok(None)
    }
}

/// Try to parse input as YAML and process it
fn try_parse_as_yaml(
    input: &str,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<Option<()>, Box<dyn std::error::Error>> {
    // Only try YAML parsing if the input actually looks like YAML
    if !is_likely_yaml(input) {
        return Ok(None);
    }

    if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(input) {
        let json_value = serde_json::to_value(yaml_value)?;
        process_structured_value(json_value, input, dsl, writer)?;
        Ok(Some(()))
    } else {
        Ok(None)
    }
}

/// Process a structured value (JSON object/array, converted TOML/YAML)
fn process_structured_value(
    json_value: serde_json::Value,
    original_input: &str,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<(), Box<dyn std::error::Error>> {
    match &json_value {
        serde_json::Value::Array(arr) => {
            // Process each item in array
            for item in arr {
                let mut item_with_original = item.clone();
                if let serde_json::Value::Object(ref mut obj) = item_with_original {
                    obj.insert(
                        "$0".to_string(),
                        serde_json::Value::String(original_input.trim().to_string()),
                    );
                }

                process_single_value(&item_with_original, dsl, writer)?;
            }
        }
        _ => {
            // Single object/value
            let mut value_with_original = json_value.clone();
            if let serde_json::Value::Object(ref mut obj) = value_with_original {
                obj.insert(
                    "$0".to_string(),
                    serde_json::Value::String(original_input.trim().to_string()),
                );
            }

            process_single_value(&value_with_original, dsl, writer)?;
        }
    }
    Ok(())
}

/// Process a single value with filter and template/field selector
fn process_single_value(
    value: &serde_json::Value,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;

    let passes_filter = if let Some(ref filter) = dsl.filter {
        FilterEngine::evaluate(filter, value)
    } else {
        true
    };

    if passes_filter {
        if let Some(ref field_selector) = dsl.field_selector {
            if let Some(extracted) = field_selector.extract_field(value) {
                writeln!(writer, "{}", extracted)?;
            }
        } else {
            let output = if let Some(ref template) = dsl.template {
                template.render(value)
            } else {
                serde_json::to_string(value)?
            };
            writeln!(writer, "{}", output)?;
        }
    }
    Ok(())
}

/// Parse a JSON array in streaming mode to avoid loading entire array into memory
fn parse_json_array_streaming(
    input: &str,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut chars = input.chars().peekable();
    let mut depth = 0;
    let mut current_object = String::new();
    let mut in_string = false;
    let mut escape_next = false;

    // Skip the opening '['
    if chars.next() != Some('[') {
        return Err("Expected '[' at start of JSON array".into());
    }

    // Skip whitespace after opening bracket
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    loop {
        // Check if we've reached the end of the array
        if let Some(&']') = chars.peek() {
            // Process any remaining object
            if !current_object.trim().is_empty() {
                process_json_object_string(&current_object, input, dsl, writer)?;
            }
            break;
        }

        // Read characters until we have a complete JSON object
        while let Some(ch) = chars.next() {
            if escape_next {
                current_object.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => {
                    current_object.push(ch);
                    escape_next = true;
                }
                '"' => {
                    current_object.push(ch);
                    in_string = !in_string;
                }
                '{' if !in_string => {
                    current_object.push(ch);
                    depth += 1;
                }
                '}' if !in_string => {
                    current_object.push(ch);
                    depth -= 1;

                    // If we've closed all braces, we have a complete object
                    if depth == 0 {
                        // Process this object
                        process_json_object_string(&current_object, input, dsl, writer)?;
                        current_object.clear();

                        // Skip whitespace and comma
                        while let Some(&next_ch) = chars.peek() {
                            if next_ch.is_whitespace() || next_ch == ',' {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        break;
                    }
                }
                _ => {
                    current_object.push(ch);
                }
            }
        }
    }

    Ok(())
}

/// Process a single JSON object from a string
fn process_json_object_string(
    object_str: &str,
    original_input: &str,
    dsl: &ParsedDSL,
    writer: &mut std::io::StdoutLock,
) -> Result<(), Box<dyn std::error::Error>> {
    let trimmed = object_str.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    // Parse the individual object
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let mut value_with_original = json_value;

        // Add $0 field with original input
        if let serde_json::Value::Object(ref mut obj) = value_with_original {
            obj.insert(
                "$0".to_string(),
                serde_json::Value::String(original_input.trim().to_string()),
            );
        }

        process_single_value(&value_with_original, dsl, writer)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Test JSON filtering with equality comparison.
    #[test]
    fn test_json_filtering() {
        let dsl = parse_command(r#"name == "Alice""#).unwrap();

        let json_data = json!({"name": "Alice", "age": 30});

        let passes = if let Some(ref filter) = dsl.filter {
            FilterEngine::evaluate(filter, &json_data)
        } else {
            true
        };

        assert!(passes);
    }

    /// Test template rendering with named field variables.
    #[test]
    fn test_template_rendering() {
        let dsl = parse_command(r#"name == "Alice" {${name} is ${age} years old}"#).unwrap();

        let json_data = json!({"name": "Alice", "age": 30});

        if let Some(ref template) = dsl.template {
            let output = template.render(&json_data);
            assert_eq!(output, "Alice is 30 years old");
        } else {
            panic!("Expected template");
        }
    }

    /// Test CSV data conversion to JSON format.
    #[test]
    fn test_csv_conversion() {
        use parsm::StreamingParser;

        let mut parser = StreamingParser::new();
        let csv_line = "Alice,30,Engineer";

        let result = parser.parse_line(csv_line).unwrap();
        let json_value = convert_parsed_line_to_json(result, csv_line).unwrap();

        assert_eq!(json_value["field_0"], "Alice");
        assert_eq!(json_value["field_1"], "30");
        assert_eq!(json_value["field_2"], "Engineer");
    }

    /// Test field selection parsing and extraction.
    #[test]
    fn test_field_selection() {
        let dsl = parse_command("\"State\"").unwrap();

        // Test that we have a field selector and no filter/template
        assert!(dsl.field_selector.is_some());
        assert!(dsl.filter.is_none());
        assert!(dsl.template.is_none());

        let field_selector = dsl.field_selector.unwrap();
        assert_eq!(field_selector.parts, vec!["State"]);

        let json_data = json!({
            "Id": "123",
            "State": {
                "Status": "running",
                "Running": true,
                "Pid": 2034
            },
            "Name": "container"
        });

        let extracted = field_selector.extract_field(&json_data).unwrap();
        let parsed_extracted: serde_json::Value = serde_json::from_str(&extracted).unwrap();

        assert_eq!(parsed_extracted["Status"], "running");
        assert_eq!(parsed_extracted["Running"], true);
        assert_eq!(parsed_extracted["Pid"], 2034);
    }

    /// Test nested field selection (e.g., "State.Status").
    #[test]
    fn test_nested_field_selection() {
        let dsl = parse_command("\"State.Status\"").unwrap();

        assert!(dsl.field_selector.is_some());
        let field_selector = dsl.field_selector.unwrap();
        assert_eq!(field_selector.parts, vec!["State", "Status"]);

        let json_data = json!({
            "State": {
                "Status": "running",
                "Running": true
            }
        });

        let extracted = field_selector.extract_field(&json_data).unwrap();
        assert_eq!(extracted, "running");
    }

    /// Test field selection behavior when field doesn't exist.
    #[test]
    fn test_field_selection_not_found() {
        let dsl = parse_command("\"NonExistent\"").unwrap();
        let field_selector = dsl.field_selector.unwrap();

        let json_data = json!({
            "State": {
                "Status": "running"
            }
        });

        let result = field_selector.extract_field(&json_data);
        assert!(result.is_none());
    }
}
