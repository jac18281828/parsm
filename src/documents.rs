//! YAML and TOML, read to the end of input as whole documents.

use serde_json::Value;

use crate::detect::Format;
use crate::lines::Line;
use crate::record::Record;
use crate::records::RecordError;

/// Records parsed from a whole input.
pub(crate) type Parsed = Vec<Result<Record, RecordError>>;

/// A document's text and the input line its text starts on.
struct Document {
    first_line: usize,
    text: String,
}

/// A document failure: the input line and the reason.
type Failure = (usize, String);

/// Parse `lines` as YAML or TOML. `None` when the first document fails and
/// the format was guessed; a forced format reports the failure instead.
pub(crate) fn read(format: Format, lines: &[Line], forced: bool) -> Option<Parsed> {
    let documents = match format {
        Format::Toml => vec![whole_input(lines)],
        _ => yaml_documents(lines),
    };
    let mut parsed = Vec::with_capacity(documents.len());
    for (index, document) in documents.iter().enumerate() {
        let value = match format {
            Format::Toml => toml_value(document),
            _ => yaml_value(document),
        };
        match value {
            Ok(value) => parsed.push(Ok(Record::value(value, document.text.trim()))),
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

fn whole_input(lines: &[Line]) -> Document {
    Document {
        first_line: lines.first().map_or(1, |line| line.number),
        text: join(lines.iter().map(|line| line.text.as_str())),
    }
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
        documents.push(Document {
            first_line,
            text: join(current.drain(..)),
        });
        if rest.is_empty() {
            first_line = line.number + 1;
        } else {
            first_line = line.number;
            current.push(rest);
        }
    }
    documents.push(Document {
        first_line,
        text: join(current.drain(..)),
    });
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
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&document.text).map_err(|error| {
        let offset = error
            .location()
            .map_or(0, |location| location.line().saturating_sub(1));
        (document.first_line + offset, error.to_string())
    })?;
    serde_json::to_value(yaml).map_err(|error| (document.first_line, error.to_string()))
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
        assert!(
            sources[1]
                .clone()
                .unwrap_err()
                .starts_with("failed to parse line")
        );
        assert_eq!(sources[2], Ok("a: 3".to_string()));
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
