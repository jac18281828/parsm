use clap::{Arg, ArgMatches, Command};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use tracing::debug;

use parsm::{Action, Format, ParsedDSL, parse_command, parse_separate_expressions, process};

/// The format flags: argument id, long flag and the format it forces.
const FORMAT_FLAGS: [(&str, &str, Format); 6] = [
    ("format-json", "json", Format::Json),
    ("format-yaml", "yaml", Format::Yaml),
    ("format-csv", "csv", Format::Csv),
    ("format-toml", "toml", Format::Toml),
    ("format-logfmt", "logfmt", Format::Logfmt),
    ("format-text", "text", Format::Text),
];

/// Main entry point for the parsm command-line tool.
///
/// Parsm is a multi-format data processor that understands structured text better than sed or awk.
/// It can parse JSON, CSV, TOML, YAML, logfmt, and plain text, applying filters and templates
/// to transform and extract data.
fn main() {
    // Initialize tracing subscriber. RUST_LOG is read once; a value that
    // fails to parse falls back to the default filter rather than aborting.
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "parsm=warn".to_string());
    let env_filter = tracing_subscriber::EnvFilter::try_new(&rust_log).unwrap_or_else(|_| {
        eprintln!("Warning: ignoring invalid RUST_LOG value '{rust_log}', using parsm=warn");
        tracing_subscriber::EnvFilter::new("parsm=warn")
    });
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(io::stderr)
        .init();

    debug!("Starting parsm");

    let matches = cli().get_matches();

    if matches.get_flag("help-examples") {
        print_usage_examples();
        return;
    }

    let forced_format = forced_format(&matches);

    let filter_expr = matches.get_one::<String>("filter");
    let template_expr = matches.get_one::<String>("template");

    // Determine the processing mode once: either plain format conversion (no
    // expression given) or a parsed DSL (field selector, filter, template, or both).
    let mode = match (filter_expr, template_expr) {
        (Some(filter), Some(template)) if !filter.trim().is_empty() => {
            match parse_separate_expressions(Some(filter), Some(template)) {
                Ok(dsl) => ProcessingMode::Filter(dsl),
                Err(e) => {
                    eprintln!("Error parsing filter and template expression: {e}");
                    std::process::exit(1);
                }
            }
        }
        (Some(_), Some(template)) => match parse_separate_expressions(None, Some(template)) {
            Ok(dsl) => ProcessingMode::Filter(dsl),
            Err(e) => {
                eprintln!("Error parsing template expression: {e}");
                std::process::exit(1);
            }
        },
        (Some(filter), None) => match parse_command(filter) {
            Ok(dsl) => ProcessingMode::Filter(dsl),
            Err(e) => {
                eprintln!("Error parsing expression: {e}");
                std::process::exit(1);
            }
        },
        (None, Some(template)) => match parse_separate_expressions(None, Some(template)) {
            Ok(dsl) => ProcessingMode::Filter(dsl),
            Err(e) => {
                eprintln!("Error parsing template expression: {e}");
                std::process::exit(1);
            }
        },
        (None, None) => ProcessingMode::Convert,
    };

    // Build the ordered list of input sources: the `-f` values in order, or a
    // single implicit stdin source when `-f` was not given at all.
    let file_args: Vec<String> = matches
        .get_many::<String>("file")
        .map(|values| values.cloned().collect())
        .unwrap_or_default();
    let sources: Vec<String> = if file_args.is_empty() {
        vec!["-".to_string()]
    } else {
        file_args
    };

    let action = match &mode {
        ProcessingMode::Convert => Action::Convert,
        ProcessingMode::Filter(dsl) => Action::Evaluate(dsl),
    };
    let stdout = io::stdout();

    for source in &sources {
        let reader: Box<dyn BufRead> = if source == "-" {
            Box::new(io::stdin().lock())
        } else {
            let file = File::open(source).unwrap_or_else(|e| {
                eprintln!("Error: cannot open file '{source}': {e}");
                std::process::exit(1);
            });
            Box::new(BufReader::new(file))
        };

        debug!("reading {source}");
        if let Err(e) = process(reader, forced_format, action, &mut stdout.lock()) {
            eprintln!("Error processing stream from '{source}': {e}");
            std::process::exit(1);
        }
    }
}

/// The command-line interface.
fn cli() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Understands structured text better than sed or awk")
        .arg(
            Arg::new("filter")
                .help("Expression: field selector, filter, template, or filter+template (optional)")
                .value_name("EXPR")
                .index(1),
        )
        .arg(
            Arg::new("template")
                .help("Template expression for output formatting (optional)")
                .value_name("TEMPLATE")
                .index(2),
        )
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("Read input from FILE instead of stdin (repeatable; '-' = stdin)")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("help-examples")
                .long("examples")
                .help("Show usage examples")
                .action(clap::ArgAction::SetTrue),
        )
        .args(FORMAT_FLAGS.map(|(id, long, format)| {
            Arg::new(id)
                .long(long)
                .help(format!(
                    "Read input as {}, skipping detection",
                    format.name()
                ))
                .action(clap::ArgAction::SetTrue)
        }))
}

/// The format named by the first format flag given.
fn forced_format(matches: &ArgMatches) -> Option<Format> {
    FORMAT_FLAGS
        .iter()
        .find(|(id, _, _)| matches.get_flag(id))
        .map(|&(_, _, format)| format)
}

/// The resolved processing mode, computed once from the CLI expression arguments.
enum ProcessingMode {
    /// No expression given: write each record as a line of JSON.
    Convert,
    /// A parsed DSL (field selector, filter, template, or a combination).
    Filter(ParsedDSL),
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
        r#"  echo '{{"name": "Alice", "age": 30}}' | parsm 'age > 25 [${{name}} is ${{age}} years old]'"#
    );
    println!();
    println!("  # Filter and format output (separate arguments):");
    println!(
        r#"  echo '{{"name": "Alice", "age": 30}}' | parsm 'age > 25' '[${{name}} is ${{age}} years old]'"#
    );
    println!();
    println!("  # Simple template variables:");
    println!(r#"  echo '{{"name": "Alice", "age": 30}}' | parsm '[$name is $age years old]'"#);
    println!();
    println!("  # Include original input with $0:");
    println!(r#"  echo 'Alice,30' | parsm '[${{0}} → ${{field_0}} is ${{field_1}}]'"#);
    println!();
    println!("  # Filter CSV data (fields accessible as field_0, field_1, etc.):");
    println!(
        r#"  echo 'Alice,30,Engineer' | parsm 'field_1 > "25" [${{field_0}} - ${{field_2}}]'"#
    );
    println!();
    println!("  # Filter logfmt logs:");
    println!(
        r#"  echo 'level=error msg="DB error" service=api' | parsm 'level == "error" [[${{level}}] ${{msg}}]'"#
    );
    println!();
    println!("  # String operations:");
    println!(r#"  echo '{{"name": "Alice"}}' | parsm 'name *= "lic"'  # contains"#);
    println!(r#"  echo '{{"name": "Alice"}}' | parsm 'name ^= "Al"'   # starts with"#);
    println!(r#"  echo '{{"name": "Alice"}}' | parsm 'name $= "ice"'  # ends with"#);
    println!(r#"  echo '{{"name": "Alice"}}' | parsm 'name ~= "A.*e"' # regex match"#);
    println!();
    println!("  # Complex conditions:");
    println!(r#"  parsm 'name == "Alice" && age > 25 [${{name}}: active]'"#);
    println!();
    println!("  # Read input from a file instead of stdin (-f is repeatable, '-' means stdin):");
    println!(r#"  parsm -f package.json 'name'"#);
    println!();
    println!("  # Convert to JSON, one line per record (no expression):");
    println!("  echo 'name: Alice' | parsm  # {{\"name\":\"Alice\"}}");
    println!();
    println!("  # Force the input format (at most one flag; skips detection):");
    println!(r#"  echo 'Alice,30' | parsm --csv '[${{field_0}} is ${{field_1}}]'"#);
    println!(r#"  echo 'level=error msg=timeout' | parsm --logfmt 'level == "error"'"#);
    println!("  echo 'name: Alice' | parsm --yaml 'name'");
    println!();
    println!("OPERATORS:");
    println!("  ==, !=, <, <=, >, >=        # Comparison");
    println!(
        "  *=, ^=, $=, ~=              # String operations (contains, starts with, ends with, regex)"
    );
    println!("  &&, ||, !                   # Boolean logic");
    println!();
    println!("FIELD ACCESS:");
    println!("  name                        # Field selection (bare identifier)");
    println!("  \"name\"                      # Field selection (quoted)");
    println!("  user.email                  # Nested field");
    println!("  field_0, field_1            # CSV columns");
    println!("  word_0, word_1              # Text words");
    println!();
    println!("TEMPLATE FORMATS:");
    println!("  [template content]          # Bracket format (preferred)");
    println!("  {{template content}}          # Brace format (alternative)");
    println!();
    println!("TEMPLATE VARIABLES:");
    println!("  ${{0}}                        # Entire original input");
    println!("  ${{field_0}}, ${{field_1}}      # CSV columns (0-based)");
    println!("  ${{word_0}}, ${{word_1}}        # Text words (0-based)");
    println!("  $name, ${{user.email}}        # Named fields ($simple or ${{complex}})");
    println!("  $100                        # Literal dollar amounts (invalid variable names)");
    println!();
    println!("FORMAT FLAGS:");
    println!("  --json                      # Read input as JSON, skipping detection");
    println!("  --yaml                      # Read input as YAML, skipping detection");
    println!("  --csv                       # Read input as CSV, skipping detection");
    println!("  --toml                      # Read input as TOML, skipping detection");
    println!("  --logfmt                    # Read input as logfmt, skipping detection");
    println!("  --text                      # Read input as text, skipping detection");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    use parsm::{FilterEngine, filter::TemplateItem};

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
        // Using just the template part to ensure it works properly
        let dsl = parse_command(r#"{${name} is ${age} years old}"#).unwrap();

        let json_data = json!({"name": "Alice", "age": 30});

        if let Some(ref template) = dsl.template {
            let output = template.render(&json_data);
            assert_eq!(output, "Alice is 30 years old");
        } else {
            panic!("Expected template");
        }

        // For combined filter + template expressions, we would need a more complex setup
        // but that's not needed for this simple rendering test
    }

    /// Test CSV fields reach templates as field_0, field_1, ...
    #[test]
    fn test_csv_conversion() {
        let dsl = parse_command("[${field_0}|${field_1}|${field_2}]").unwrap();
        let mut output = Vec::new();
        process(
            std::io::Cursor::new("Alice,30,Engineer"),
            None,
            Action::Evaluate(&dsl),
            &mut output,
        )
        .unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "Alice|30|Engineer\n");
    }

    /// A format flag names the format it forces.
    #[test]
    fn test_format_flag_forces_format() {
        let matches = cli().try_get_matches_from(["parsm", "--csv"]).unwrap();
        assert_eq!(forced_format(&matches), Some(Format::Csv));
        let matches = cli().try_get_matches_from(["parsm"]).unwrap();
        assert_eq!(forced_format(&matches), None);
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

    /// Test debug output of template parsing.
    #[test]
    fn debug_template_parsing() {
        let dsl = parse_command(r#"{${name} is ${age} years old}"#).unwrap();

        if let Some(ref template) = dsl.template {
            println!("Template items: {:?}", template.items);
            let json_data = json!({"name": "Alice", "age": 30});
            let output = template.render(&json_data);
            println!("Template output: '{output}'");
        } else {
            panic!("Expected template");
        }
    }

    /// Test detailed debug output of template rendering.
    #[test]
    fn debug_template_rendering_detailed() {
        let dsl = parse_command(r#"{${name} is ${age} years old}"#).unwrap();

        if let Some(ref template) = dsl.template {
            println!("Template items: {:?}", template.items);
            let json_data = json!({"name": "Alice", "age": 30});

            let mut result = String::new();
            for (i, item) in template.items.iter().enumerate() {
                match item {
                    TemplateItem::Field(field) => {
                        if let Some(value) = field.get_value(&json_data) {
                            let formatted = value.to_string();
                            println!("Item {i}: Field({field:?}) -> '{formatted}'");
                            result.push_str(&formatted);
                        }
                    }
                    TemplateItem::Literal(text) => {
                        println!("Item {i}: Literal -> '{text}'");
                        result.push_str(text);
                    }
                    TemplateItem::Conditional { .. } => {
                        println!("Item {i}: Conditional");
                    }
                }
            }

            println!("Manual result: '{result}'");
            let template_result = template.render(&json_data);
            println!("Template result: '{template_result}'");
        } else {
            panic!("Expected template");
        }
    }

    /// Test interpolated template syntax
    #[test]
    fn test_interpolated_template() {
        let dsl = parse_command(r#"[Hello ${name}, you are ${age} years old]"#).unwrap();

        if let Some(ref template) = dsl.template {
            println!("Interpolated template items: {:?}", template.items);
            let json_data = json!({"name": "Alice", "age": 30});
            let output = template.render(&json_data);
            println!("Interpolated output: '{output}'");
        } else {
            println!("No template found");
        }
    }

    /// Test a template with explicit spacing
    #[test]
    fn debug_simple_template() {
        let dsl = parse_command(r#"{${name}_is_${age}_years_old}"#).unwrap();

        if let Some(ref template) = dsl.template {
            println!("Simple template items: {:?}", template.items);
            let json_data = json!({"name": "Alice", "age": 30});
            let output = template.render(&json_data);
            println!("Simple output: '{output}'");
        }
    }
}
