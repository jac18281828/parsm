//! The one format detector.
//!
//! Detection runs on the input, never on the expression. It applies the
//! precedence table below to the first non-blank line that does not start
//! with `#` (the probe); the first row that matches chooses the format.
//!
//! | Row | Format | Chosen when |
//! |---|---|---|
//! | 1 | JSON | the first JSON value parses (it may span lines) and only whitespace or further `{…}`/`[…]` values follow it on its last line |
//! | 2 | logfmt | every whitespace-separated token is `key=value` and the line parses as logfmt |
//! | 3 | TOML | a `key = value` line, or a `[table]` line followed by one, that parses |
//! | 4 | YAML | a block mapping `key: value`, a `- item`, `---`, or a `{…}` flow map |
//! | 5 | CSV | the line has commas and parses as a CSV row |
//! | 6 | text | everything else |
//!
//! Every line detection reads is replayed to the chosen format.

use std::fmt;
use std::io::{self, BufRead, Cursor};

use crate::json_stream::{self, Opening};
use crate::lines::{Line, LineSource};
use crate::parse::{parse_csv_line, parse_logfmt};

/// An input format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Json,
    Logfmt,
    Toml,
    Yaml,
    Csv,
    Text,
}

impl Format {
    /// Lower-case name, as in the format flags.
    pub fn name(self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Logfmt => "logfmt",
            Format::Toml => "toml",
            Format::Yaml => "yaml",
            Format::Csv => "csv",
            Format::Text => "text",
        }
    }

    /// This format's row in the precedence table.
    pub fn precedence_row(self) -> u8 {
        match self {
            Format::Json => 1,
            Format::Logfmt => 2,
            Format::Toml => 3,
            Format::Yaml => 4,
            Format::Csv => 5,
            Format::Text => 6,
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The format the precedence table chooses for `input`.
///
/// A YAML or TOML guess whose first document fails to parse is read as text
/// by [`crate::process`]; this reports the guess.
pub fn detect_format(input: &str) -> Format {
    let mut source = LineSource::new(Cursor::new(input.as_bytes()));
    // Reading from memory cannot fail; text is the table's last row.
    detect(&mut source).map_or(Format::Text, |detection| detection.format)
}

/// The chosen format; for JSON, its first value, already parsed.
pub(crate) struct Detection {
    pub(crate) format: Format,
    pub(crate) opening: Option<Opening>,
}

impl From<Format> for Detection {
    fn from(format: Format) -> Self {
        Self {
            format,
            opening: None,
        }
    }
}

/// Choose the format of `source`, replaying every line read that the chosen
/// format has not yet consumed.
pub(crate) fn detect<R: BufRead>(source: &mut LineSource<R>) -> io::Result<Detection> {
    let mut read = Vec::new();
    let mut detection = choose(source, &mut read)?;
    match &mut detection.opening {
        Some(opening) => source.replay(std::mem::take(&mut opening.rest)),
        None => source.replay(read),
    }
    Ok(detection)
}

fn choose<R: BufRead>(source: &mut LineSource<R>, read: &mut Vec<Line>) -> io::Result<Detection> {
    let Some(probe) = next_meaningful(source, read)? else {
        return Ok(Format::Text.into());
    };
    if let Some(opening) = opens_json(source, read)? {
        return Ok(Detection {
            format: Format::Json,
            opening: Some(opening),
        });
    }
    let probe = probe.text;
    let format = if parse_logfmt(&probe).is_some() {
        Format::Logfmt
    } else if opens_toml(&probe, source, read)? {
        Format::Toml
    } else if opens_yaml(&probe) {
        Format::Yaml
    } else if opens_csv(&probe) {
        Format::Csv
    } else {
        Format::Text
    };
    Ok(format.into())
}

/// Read up to and including the next line that is neither blank nor a `#`
/// line.
fn next_meaningful<R: BufRead>(
    source: &mut LineSource<R>,
    read: &mut Vec<Line>,
) -> io::Result<Option<Line>> {
    while let Some(line) = source.next_line()? {
        read.push(line.clone());
        if !line.is_blank() && !line.is_comment() {
            return Ok(Some(line));
        }
    }
    Ok(None)
}

/// Row 1. The probe is the last line in `read`.
fn opens_json<R: BufRead>(
    source: &mut LineSource<R>,
    read: &mut Vec<Line>,
) -> io::Result<Option<Opening>> {
    let Some(probe) = read.pop() else {
        return Ok(None);
    };
    json_stream::opens_json_input(probe, source, read)
}

/// Row 3.
fn opens_toml<R: BufRead>(
    probe: &str,
    source: &mut LineSource<R>,
    read: &mut Vec<Line>,
) -> io::Result<bool> {
    if !is_table_header(probe) {
        return Ok(probe.contains('=') && toml::from_str::<toml::Table>(probe).is_ok());
    }
    let pair = read
        .iter()
        .filter(|line| !line.is_blank() && !line.is_comment())
        .nth(1)
        .cloned();
    let pair = match pair {
        Some(line) => Some(line),
        None => next_meaningful(source, read)?,
    };
    Ok(pair.is_some_and(|pair| {
        pair.text.contains('=')
            && !is_table_header(&pair.text)
            && toml::from_str::<toml::Table>(&format!("{probe}\n{}", pair.text)).is_ok()
    }))
}

fn is_table_header(line: &str) -> bool {
    let line = line.trim();
    line.starts_with('[') && line.ends_with(']')
}

/// Row 4.
fn opens_yaml(probe: &str) -> bool {
    let line = probe.trim();
    line == "---"
        || line.starts_with("--- ")
        || line == "-"
        || line.starts_with("- ")
        || (line.starts_with('{') && line.ends_with('}'))
        || is_block_mapping(line)
}

/// A `key:` followed by whitespace or the end of the line, where the key is
/// quoted or a plain scalar.
fn is_block_mapping(line: &str) -> bool {
    let rest = match line.chars().next() {
        Some(quote @ ('"' | '\'')) => match line[1..].find(quote) {
            Some(close) => &line[close + 2..],
            None => return false,
        },
        Some(first) if !"[]{},#&*!|>%@`".contains(first) => {
            return plain_key_end(line).is_some();
        }
        _ => return false,
    };
    ends_key(rest)
}

/// Byte offset of the `:` ending a plain key.
fn plain_key_end(line: &str) -> Option<usize> {
    line.char_indices()
        .find(|&(index, ch)| ch == ':' && ends_key(&line[index..]))
        .map(|(index, _)| index)
        .filter(|&index| index > 0 && !line[..index].contains(" #"))
}

/// Whether `rest` starts with `:` followed by whitespace or nothing.
fn ends_key(rest: &str) -> bool {
    rest.strip_prefix(':')
        .is_some_and(|after| after.is_empty() || after.starts_with(char::is_whitespace))
}

/// Row 5.
fn opens_csv(probe: &str) -> bool {
    let commas = probe.matches(',').count();
    commas > 0 && probe.trim().len() > commas && parse_csv_line(probe).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_table_examples() {
        let cases = [
            ("42", Format::Json),
            ("true", Format::Json),
            ("\"hi\"", Format::Json),
            ("[1,2]", Format::Json),
            ("{\n  \"a\": 1\n}", Format::Json),
            ("{\"a\":1} {\"a\":2}", Format::Json),
            ("a=1", Format::Logfmt),
            ("x=1 y=2", Format::Logfmt),
            ("a = 1", Format::Toml),
            ("name = \"Alice\"", Format::Toml),
            ("[server]\nport = 80", Format::Toml),
            ("a: 1", Format::Yaml),
            ("- a", Format::Yaml),
            ("---\na: 1", Format::Yaml),
            ("{a: 1}", Format::Yaml),
            ("user:\n  name: Alice", Format::Yaml),
            ("\"quoted key\": 1", Format::Yaml),
            ("a,b", Format::Csv),
            ("1,2,3", Format::Csv),
            ("\"Smith, John\",30", Format::Csv),
            ("hello", Format::Text),
            ("a:1", Format::Text),
            ("[a]", Format::Text),
            ("2024-01-01", Format::Text),
            ("200 OK", Format::Text),
            ("1 2 3", Format::Text),
            ("http://example.com", Format::Text),
            ("{\"text\": \"Hello\nWorld\"}", Format::Text),
            ("", Format::Text),
        ];
        for (input, expected) in cases {
            assert_eq!(detect_format(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn leading_comments_and_blanks_are_held() {
        assert_eq!(detect_format("# cfg\n\nname: Alice"), Format::Yaml);
        assert_eq!(detect_format("# cfg\n{\"a\":1}"), Format::Json);
    }

    #[test]
    fn malformed_json_falls_through() {
        assert_eq!(
            detect_format("{\"name\": \"Invalid JSON\"\n{\"name\": \"x\"}"),
            Format::Text
        );
        assert_eq!(detect_format("{\"a\":1,\n\"b\": 2, x"), Format::Csv);
    }

    #[test]
    fn table_header_needs_a_following_pair() {
        assert_eq!(detect_format("[a]\n[b]"), Format::Text);
        assert_eq!(detect_format("[a]\n# c\nkey = 1"), Format::Toml);
    }

    #[test]
    fn every_line_read_is_replayed() {
        let mut source = LineSource::new(Cursor::new("# c\n{\n\"a\": 1\n}\nnext\n"));
        assert_eq!(detect(&mut source).unwrap().format, Format::Json);
        let lines: Vec<String> = source
            .read_to_end()
            .unwrap()
            .into_iter()
            .map(|l| l.text)
            .collect();
        assert_eq!(lines, vec!["next"]);

        let mut source = LineSource::new(Cursor::new("# c\nname: Alice\nnext\n"));
        assert_eq!(detect(&mut source).unwrap().format, Format::Yaml);
        let lines: Vec<String> = source
            .read_to_end()
            .unwrap()
            .into_iter()
            .map(|l| l.text)
            .collect();
        assert_eq!(lines, vec!["# c", "name: Alice", "next"]);
    }
}
