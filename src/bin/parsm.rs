// main.rs - Fixed integrated application

use clap::{Arg, Command};
use std::io;

use parsm::{parse_command, process_stream, FilterEngine, ParsedDSL, ParsedLine};

fn main() {
    let matches = Command::new("parsm")
        .version("0.1.0")
        .author("John Cairns <john@2ad.com>")
        .about("Understands structured text better than sed or awk")
        .arg(
            Arg::new("filter")
                .help("Filter and template expression")
                .value_name("EXPRESSION")
                .index(1),
        )
        .arg(
            Arg::new("help-examples")
                .long("examples")
                .help("Show usage examples")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Show examples if requested
    if matches.get_flag("help-examples") {
        print_usage_examples();
        return;
    }

    if let Some(filter_expr) = matches.get_one::<String>("filter") {
        // Parse the filter and template expression
        let parsed_dsl = match parse_command(filter_expr) {
            Ok(dsl) => dsl,
            Err(e) => {
                eprintln!("Error parsing filter expression: {}", e);
                std::process::exit(1);
            }
        };

        // Process stream with filtering
        if let Err(e) = process_stream_with_filter(parsed_dsl) {
            eprintln!("Error processing stream: {}", e);
            std::process::exit(1);
        }
    } else {
        // No filter provided, just convert formats
        let stdin = io::stdin();
        let stdout = io::stdout();

        if let Err(e) = process_stream(stdin.lock(), stdout) {
            eprintln!("Error processing stream: {}", e);
            std::process::exit(1);
        }
    }
}

fn process_stream_with_filter(dsl: ParsedDSL) -> Result<(), Box<dyn std::error::Error>> {
    use parsm::StreamingParser;
    use std::io::{BufRead, Write};

    let stdin = io::stdin();
    let mut parser = StreamingParser::new();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut line_count = 0;

    for line_result in stdin.lock().lines() {
        let line = line_result?;
        line_count += 1;

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Parse the line using our multi-format parser
        match parser.parse_line(&line) {
            Ok(parsed_line) => {
                // Convert parsed line to JSON for filtering
                let json_value = convert_parsed_line_to_json(parsed_line, &line)?;

                // Apply filter if provided
                let passes_filter = if let Some(ref filter) = dsl.filter {
                    FilterEngine::evaluate(filter, &json_value)
                } else {
                    true // No filter means everything passes
                };

                if passes_filter {
                    // Apply template if provided, otherwise output as JSON
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

// Convert ParsedLine to JSON Value for uniform processing
fn convert_parsed_line_to_json(
    parsed_line: ParsedLine,
    original_input: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use serde_json::Value;

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

// Example usage demonstration
fn print_usage_examples() {
    println!("parsm - Multi-format data processor");
    println!();
    println!("EXAMPLES:");
    println!();
    println!("  # Filter JSON by field value:");
    println!(r#"  echo '{{"name": "Alice", "age": 30}}' | parsm 'name == "Alice"'"#);
    println!();
    println!("  # Filter and format output:");
    println!(
        r#"  echo '{{"name": "Alice", "age": 30}}' | parsm 'age > 25' '$name is $age years old'"#
    );
    println!();
    println!("  # Filter CSV data (fields accessible as field_0, field_1, etc.):");
    println!(r#"  echo 'Alice,30,Engineer' | parsm 'field_1 > "25"' '$field_0 - $field_2'"#);
    println!();
    println!("  # Filter logfmt logs:");
    println!(
        r#"  echo 'level=error msg="DB error" service=api' | parsm 'level == "error"' '[$level] $msg'"#
    );
    println!();
    println!("  # Complex conditions:");
    println!(r#"  parsm 'name == "Alice" && age > 25 || status == "active"' '$name: $status'"#);
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
    println!("  name                        # Simple field");
    println!("  user.email                  # Nested field");
    println!("  field_0, field_1            # CSV columns");
    println!("  word_0, word_1              # Text words");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    #[test]
    fn test_template_rendering() {
        let dsl = parse_command(r#"name == "Alice" $name is $age years old"#).unwrap();

        let json_data = json!({"name": "Alice", "age": 30});

        if let Some(ref template) = dsl.template {
            let output = template.render(&json_data);
            assert_eq!(output, "Alice is 30 years old");
        } else {
            panic!("Expected template");
        }
    }

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
}
