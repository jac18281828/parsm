//! Records to output: convert to JSON lines, or filter and render.

use std::error::Error;
use std::io::{BufRead, Write};

use crate::detect::Format;
use crate::dsl::ParsedDSL;
use crate::filter::FilterEngine;
use crate::record::Record;
use crate::records::{RecordError, Records};

/// What to write for each record.
#[derive(Clone, Copy)]
pub enum Action<'a> {
    /// One line of compact JSON per record.
    Convert,
    /// Records passing the filter, as the field selector, the template or
    /// `$0`.
    Evaluate(&'a ParsedDSL),
}

/// Read every record of `reader` and write each as `action` directs.
///
/// Each record is written and flushed as it completes, so the first record
/// never waits for the end of input. A record that fails after others have
/// parsed is reported on stderr as a warning and skipped; a failure before
/// anything parses is returned.
pub fn process<R: BufRead, W: Write>(
    reader: R,
    format: Option<Format>,
    action: Action<'_>,
    writer: &mut W,
) -> Result<(), Box<dyn Error>> {
    for item in Records::open(reader, format)? {
        match item {
            Ok(record) => write_record(&record, action, writer)?,
            Err(warning @ RecordError::Skipped { .. }) => eprintln!("Warning: {warning}"),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// Write one record as `action` directs, then flush.
pub fn write_record<W: Write>(
    record: &Record,
    action: Action<'_>,
    writer: &mut W,
) -> std::io::Result<()> {
    match action {
        Action::Convert => writeln!(writer, "{}", record.to_json())?,
        Action::Evaluate(dsl) => {
            if let Some(output) = evaluate(record, dsl) {
                writeln!(writer, "{output}")?;
            }
        }
    }
    writer.flush()
}

/// The output line for `record`, or `None` when it is filtered out or lacks
/// the selected field.
fn evaluate(record: &Record, dsl: &ParsedDSL) -> Option<String> {
    let view = record.view();
    if let Some(filter) = &dsl.filter
        && !FilterEngine::evaluate(filter, &view)
    {
        return None;
    }
    if let Some(selector) = &dsl.field_selector {
        return selector.extract_field(&view);
    }
    Some(match &dsl.template {
        Some(template) => template.render(&view),
        None => record.source().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::{parse_command, parse_separate_expressions};
    use std::io::Cursor;

    fn run(input: &str, format: Option<Format>, action: Action<'_>) -> String {
        let mut output = Vec::new();
        process(Cursor::new(input), format, action, &mut output).unwrap();
        String::from_utf8(output).unwrap()
    }

    fn evaluate_with(expression: &str, input: &str) -> String {
        let dsl = parse_command(expression).unwrap();
        run(input, None, Action::Evaluate(&dsl))
    }

    #[test]
    fn convert_writes_compact_json_per_record() {
        assert_eq!(
            run("name: Alice\nage: 30", None, Action::Convert),
            "{\"name\":\"Alice\",\"age\":30}\n"
        );
        assert_eq!(
            run("name,age\nTom,45", None, Action::Convert),
            "{\"name\":\"Tom\",\"age\":\"45\"}\n"
        );
    }

    #[test]
    fn filter_without_template_writes_source() {
        let dsl = parse_separate_expressions(Some("a > 1"), None).unwrap();
        assert!(dsl.template.is_none());
        assert_eq!(run("a=2\na=1", None, Action::Evaluate(&dsl)), "a=2\n");
        assert_eq!(evaluate_with("a > 1", "a=2\na=1"), "a=2\n");
    }

    #[test]
    fn missing_selected_field_writes_nothing() {
        assert_eq!(evaluate_with("nope", "{\"a\":1}\n{\"nope\":2}"), "2\n");
    }

    #[test]
    fn forced_format_failure_is_returned() {
        let mut output = Vec::new();
        let result = process(
            Cursor::new("hello world"),
            Some(Format::Json),
            Action::Convert,
            &mut output,
        );
        assert!(result.is_err());
        assert!(output.is_empty());
    }
}
