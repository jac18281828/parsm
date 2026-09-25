//! YAML and TOML, read to the end of input as whole documents.

use serde_json::Value;

use crate::detect::Format;
use crate::lines::{Line, UNDECODABLE};
use crate::record::Record;
use crate::records::RecordError;

/// Records parsed from a whole input.
pub(crate) type Parsed = Vec<Result<Record, RecordError>>;

/// A document's text and the input lines its text spans.
struct Document {
    first_line: usize,
    last_line: usize,
    text: String,
}

impl Document {
    fn new(first_line: usize, text: String) -> Self {
        let last_line = first_line + text.lines().count().saturating_sub(1);
        Self {
            first_line,
            last_line,
            text,
        }
    }
}

/// A document failure: the input line and the reason.
type Failure = (usize, String);

/// Parse `lines` as YAML or TOML. `None` when the first document fails and
/// the format was guessed; a forced format reports the failure instead.
///
/// Lines that are not valid UTF-8 are reported and left out; one before the
/// first content line fails the input.
pub(crate) fn read(format: Format, lines: &[Line], forced: bool) -> Option<Parsed> {
    let (decoded, undecodable): (Vec<Line>, Vec<Line>) =
        lines.iter().cloned().partition(|line| line.decoded);
    let first_content = decoded
        .iter()
        .find(|line| !line.is_blank() && !line.is_comment())
        .map_or(usize::MAX, |line| line.number);
    if let Some(line) = undecodable.first()
        && line.number < first_content
    {
        let failure = RecordError::parse(false, format, line.number, UNDECODABLE);
        return Some(vec![Err(failure)]);
    }
    let documents = match format {
        Format::Toml => vec![whole_input(&decoded)],
        _ => yaml_documents(&decoded),
    };
    let mut parsed: Parsed = undecodable
        .iter()
        .map(|line| Err(RecordError::parse(true, format, line.number, UNDECODABLE)))
        .collect();
    for (index, document) in documents.iter().enumerate() {
        match document_records(format, document) {
            Ok(records) => parsed.extend(records.into_iter().map(Ok)),
            Err(_) if index == 0 && !forced => return None,
            Err((line, reason)) => {
                parsed.push(Err(RecordError::parse(index > 0, format, line, reason)));
                if index == 0 {
                    break;
                }
            }
        }
    }
    Some(parsed)
}

fn document_records(format: Format, document: &Document) -> Result<Vec<Record>, Failure> {
    match format {
        Format::Toml => {
            let value = toml_value(document)?;
            Ok(vec![Record::value(value, document.text.trim())])
        }
        _ => yaml_records(document),
    }
}

/// A top-level sequence yields one record per item; any other document is
/// one record.
fn yaml_records(document: &Document) -> Result<Vec<Record>, Failure> {
    let items = match yaml_value(document)? {
        Value::Array(items) => items,
        value => return Ok(vec![Record::value(value, document.text.trim())]),
    };
    let sources = sequence_item_sources(&document.text, items.len())
        .unwrap_or_else(|| items.iter().map(Value::to_string).collect());
    Ok(items
        .into_iter()
        .zip(sources)
        .map(|(item, source)| Record::value(item, source))
        .collect())
}

/// Each top-level item's text, its `- ` marker included; `None` when the
/// markers do not match `count` items, as in a flow sequence.
fn sequence_item_sources(text: &str, count: usize) -> Option<Vec<String>> {
    let mut items: Vec<Vec<&str>> = Vec::new();
    let mut indent = None;
    for line in text.lines() {
        let content = line.trim_start();
        let depth = line.len() - content.len();
        let marker = content == "-" || content.starts_with("- ");
        if marker && indent.is_none_or(|indent| indent == depth) {
            indent = Some(depth);
            items.push(vec![line]);
        } else if let Some(item) = items.last_mut() {
            item.push(line);
        }
    }
    (items.len() == count).then(|| {
        items
            .iter()
            .map(|lines| lines.join("\n").trim_end().to_string())
            .collect()
    })
}

fn whole_input(lines: &[Line]) -> Document {
    let first_line = lines.first().map_or(1, |line| line.number);
    Document::new(
        first_line,
        join(lines.iter().map(|line| line.text.as_str())),
    )
}

/// Split on `---` lines; a document's text excludes its `---` line.
/// Documents holding only blank and `#` lines are dropped.
fn yaml_documents(lines: &[Line]) -> Vec<Document> {
    let mut documents = Vec::new();
    let mut first_line = lines.first().map_or(1, |line| line.number);
    let mut current: Vec<&str> = Vec::new();
    for line in lines {
        let Some(rest) = document_marker(&line.text) else {
            current.push(&line.text);
            continue;
        };
        documents.push(Document::new(first_line, join(current.drain(..))));
        if rest.is_empty() {
            first_line = line.number + 1;
        } else {
            first_line = line.number;
            current.push(rest);
        }
    }
    documents.push(Document::new(first_line, join(current.drain(..))));
    documents.retain(has_content);
    documents
}

/// Text after a `---` document marker, or `None` when `line` is not one.
fn document_marker(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("---")?;
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(rest.trim())
    } else {
        None
    }
}

fn has_content(document: &Document) -> bool {
    document.text.lines().any(|line| {
        let line = line.trim();
        !line.is_empty() && !line.starts_with('#')
    })
}

fn join<'a>(lines: impl Iterator<Item = &'a str>) -> String {
    lines.collect::<Vec<_>>().join("\n")
}

/// YAML is read as `serde_yaml_ng::Value`, which rejects duplicate keys.
fn yaml_value(document: &Document) -> Result<Value, Failure> {
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&document.text).map_err(|error| yaml_failure(document, &error))?;
    serde_json::to_value(yaml).map_err(|error| (document.first_line, error.to_string()))
}

/// The input line of a YAML error, kept within its document, and the
/// message without serde_yaml_ng's document-relative position.
fn yaml_failure(document: &Document, error: &serde_yaml_ng::Error) -> Failure {
    let message = error.to_string();
    match error.location() {
        Some(location) => {
            let offset = location.line().saturating_sub(1);
            let line = (document.first_line + offset).min(document.last_line);
            let position = format!(" at line {} column {}", location.line(), location.column());
            (line, message.replace(&position, ""))
        }
        None => (document.first_line, message),
    }
}

fn toml_value(document: &Document) -> Result<Value, Failure> {
    let toml: toml::Value = toml::from_str(&document.text).map_err(|error| {
        let offset = error
            .span()
            .map_or(0, |span| document.text[..span.start].matches('\n').count());
        (document.first_line + offset, error.message().to_string())
    })?;
    serde_json::to_value(toml).map_err(|error| (document.first_line, error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(input: &str) -> Vec<Line> {
        input
            .lines()
            .enumerate()
            .map(|(index, text)| Line {
                number: index + 1,
                text: text.to_string(),
                decoded: true,
            })
            .collect()
    }

    fn sources(parsed: &Parsed) -> Vec<Result<String, String>> {
        parsed
            .iter()
            .map(|item| match item {
                Ok(record) => Ok(record.source().to_string()),
                Err(error) => Err(error.to_string()),
            })
            .collect()
    }

    #[test]
    fn yaml_yields_one_record_per_document() {
        let parsed = read(Format::Yaml, &lines("---\nage: 30\n---\nage: 40\n"), false).unwrap();
        assert_eq!(
            sources(&parsed),
            vec![Ok("age: 30".to_string()), Ok("age: 40".to_string())]
        );
    }

    #[test]
    fn yaml_first_document_keeps_leading_comments() {
        let parsed = read(Format::Yaml, &lines("# cfg\nname: Alice"), false).unwrap();
        assert_eq!(sources(&parsed), vec![Ok("# cfg\nname: Alice".to_string())]);
    }

    #[test]
    fn guessed_yaml_with_duplicate_keys_is_unparsable() {
        assert!(read(Format::Yaml, &lines("INFO: a\nINFO: b"), false).is_none());
    }

    #[test]
    fn forced_yaml_first_failure_is_reported() {
        let parsed = read(Format::Yaml, &lines("INFO: a\nINFO: b"), true).unwrap();
        assert!(matches!(parsed[..], [Err(RecordError::First { .. })]));
    }

    #[test]
    fn later_yaml_document_failure_is_skipped() {
        let parsed = read(Format::Yaml, &lines("a: 1\n---\na: [\n---\na: 3"), false).unwrap();
        let sources = sources(&parsed);
        assert_eq!(sources.len(), 3);
        let warning = sources[1].clone().unwrap_err();
        assert!(warning.starts_with("failed to parse line 3: "), "{warning}");
        assert!(!warning.contains(" at line "), "{warning}");
        assert_eq!(sources[2], Ok("a: 3".to_string()));
    }

    #[test]
    fn yaml_sequence_items_are_records_with_their_marker_lines() {
        let input = "# list\n- name: x\n  n: 1\n- name: y\n- plain";
        let parsed = read(Format::Yaml, &lines(input), false).unwrap();
        assert_eq!(
            sources(&parsed),
            vec![
                Ok("- name: x\n  n: 1".to_string()),
                Ok("- name: y".to_string()),
                Ok("- plain".to_string()),
            ]
        );
    }

    #[test]
    fn yaml_flow_sequence_items_use_compact_json_sources() {
        let parsed = read(Format::Yaml, &lines("[a, {b: 1}]"), true).unwrap();
        assert_eq!(
            sources(&parsed),
            vec![Ok("\"a\"".to_string()), Ok("{\"b\":1}".to_string())]
        );
    }

    #[test]
    fn undecodable_lines_warn_after_content_and_fail_before_it() {
        let mut input = lines("a: 1\nb: 2\nc: 3");
        input[1].decoded = false;
        let parsed = read(Format::Yaml, &input, false).unwrap();
        assert!(matches!(
            parsed[..],
            [Err(RecordError::Skipped { line: 2, .. }), Ok(_)]
        ));

        let mut input = lines("a: 1\nb: 2");
        input[0].decoded = false;
        let parsed = read(Format::Yaml, &input, false).unwrap();
        assert!(matches!(
            parsed[..],
            [Err(RecordError::First { line: 1, .. })]
        ));
    }

    #[test]
    fn toml_is_one_record() {
        let parsed = read(Format::Toml, &lines("a = 1\n[t]\nb = 2"), false).unwrap();
        assert_eq!(sources(&parsed), vec![Ok("a = 1\n[t]\nb = 2".to_string())]);
    }

    #[test]
    fn forced_toml_failure_names_its_line() {
        let parsed = read(Format::Toml, &lines("a = 1\nb = ["), true).unwrap();
        match &parsed[..] {
            [Err(RecordError::First { line, .. })] => assert_eq!(*line, 2),
            other => panic!("unexpected {other:?}"),
        }
    }
}
