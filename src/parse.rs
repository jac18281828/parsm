//! Single-line parsers for the line-oriented formats.

use serde_json::{Map, Value};
use std::iter::Peekable;
use std::str::Chars;

/// Parse one CSV line into its fields.
pub fn parse_csv_line(line: &str) -> Option<csv::StringRecord> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(line.as_bytes());
    reader.records().next().transpose().ok()?
}

/// Parse one logfmt line into an object of string values.
///
/// Every whitespace-separated token must be `key=value`: a key holding
/// whitespace, an empty key or a token without `=` rejects the line. Values
/// may be bare, `"quoted"` or `\"escaped-quoted\"`.
pub fn parse_logfmt(line: &str) -> Option<Value> {
    let mut map = Map::new();
    let mut chars = line.chars().peekable();

    while let Some(first) = chars.next() {
        if first.is_whitespace() {
            continue;
        }
        let key = read_key(first, &mut chars)?;
        let value = read_value(&mut chars);
        map.insert(key, Value::String(value));
    }

    if map.is_empty() {
        None
    } else {
        Some(Value::Object(map))
    }
}

/// Read a key through its `=`; `None` when the token is not `key=`.
fn read_key(first: char, chars: &mut Peekable<Chars>) -> Option<String> {
    let mut key = String::new();
    let mut current = first;
    while current != '=' {
        if current.is_whitespace() {
            return None;
        }
        key.push(current);
        current = chars.next()?;
    }
    if key.is_empty() { None } else { Some(key) }
}

fn read_value(chars: &mut Peekable<Chars>) -> String {
    match chars.peek().copied() {
        Some('\\') if starts_escaped_quote(chars) => {
            chars.next();
            chars.next();
            read_escaped_quoted(chars)
        }
        Some('"') => {
            chars.next();
            read_quoted(chars)
        }
        _ => {
            let mut value = String::new();
            while let Some(ch) = chars.next_if(|ch| !ch.is_whitespace()) {
                value.push(ch);
            }
            value
        }
    }
}

fn starts_escaped_quote(chars: &Peekable<Chars>) -> bool {
    let mut lookahead = chars.clone();
    lookahead.next();
    lookahead.peek() == Some(&'"')
}

/// Read a `"…"` value after its opening quote.
fn read_quoted(chars: &mut Peekable<Chars>) -> String {
    let mut value = String::new();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => break,
            '\\' => {
                if let Some(escaped) = chars.next() {
                    push_escape(&mut value, escaped);
                }
            }
            _ => value.push(ch),
        }
    }
    value
}

/// Read a `\"…\"` value after its opening `\"`; an unclosed value runs to
/// the end of the line.
fn read_escaped_quoted(chars: &mut Peekable<Chars>) -> String {
    let mut value = String::new();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            value.push(ch);
            continue;
        }
        if chars.next_if_eq(&'"').is_some() {
            break;
        }
        if let Some(escaped) = chars.next() {
            push_escape(&mut value, escaped);
        }
    }
    value
}

fn push_escape(value: &mut String, escaped: char) {
    match escaped {
        '"' => value.push('"'),
        '\\' => value.push('\\'),
        'n' => value.push('\n'),
        't' => value.push('\t'),
        'r' => value.push('\r'),
        other => {
            value.push('\\');
            value.push(other);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logfmt(line: &str) -> Value {
        parse_logfmt(line).expect("line should parse as logfmt")
    }

    #[test]
    fn test_logfmt_bare_and_quoted_values() {
        let value = logfmt(r#"level=info msg="Starting application" port=8080"#);
        assert_eq!(value["level"], "info");
        assert_eq!(value["msg"], "Starting application");
        assert_eq!(value["port"], "8080");
    }

    #[test]
    fn test_logfmt_rejects_tokens_without_equals() {
        assert!(parse_logfmt("hello world").is_none());
        assert!(parse_logfmt("a b=1").is_none());
        assert!(parse_logfmt("a = 1").is_none());
        assert!(parse_logfmt("=1").is_none());
        assert!(parse_logfmt("").is_none());
    }

    #[test]
    fn test_logfmt_keeps_key_order() {
        let value = logfmt("z=1 a=2");
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["z", "a"]);
    }

    #[test]
    fn test_logfmt_escaped_quotes_comprehensive() {
        let value = logfmt(r#"level=error msg=\"DB connection failed\" service=api"#);
        assert_eq!(value["level"], "error");
        assert_eq!(value["msg"], "DB connection failed");
        assert_eq!(value["service"], "api");
    }

    #[test]
    fn test_logfmt_mixed_quote_styles() {
        let value = logfmt(r#"level=info msg=\"Server starting\" port=8080 env="production""#);
        assert_eq!(value["level"], "info");
        assert_eq!(value["msg"], "Server starting");
        assert_eq!(value["port"], "8080");
        assert_eq!(value["env"], "production");
    }

    #[test]
    fn test_logfmt_escaped_quotes_with_spaces() {
        let value = logfmt(
            r#"action=login user=\"john doe\" reason=\"failed: invalid password\" attempts=3"#,
        );
        assert_eq!(value["action"], "login");
        assert_eq!(value["user"], "john doe");
        assert_eq!(value["reason"], "failed: invalid password");
        assert_eq!(value["attempts"], "3");
    }

    #[test]
    fn test_logfmt_nested_escape_sequences() {
        let value = logfmt(r#"msg=\"Error: \\server\\path\\file.txt\" status=\"failed\""#);
        assert_eq!(value["msg"], r"Error: \server\path\file.txt");
        assert_eq!(value["status"], "failed");
    }

    #[test]
    fn test_logfmt_empty_escaped_quotes() {
        let value = logfmt(r#"level=debug msg=\"\" user=system"#);
        assert_eq!(value["level"], "debug");
        assert_eq!(value["msg"], "");
        assert_eq!(value["user"], "system");
    }

    #[test]
    fn test_logfmt_malformed_escaped_quotes() {
        let value = logfmt(r#"level=error msg=\"unclosed quote service=api"#);
        assert_eq!(value["level"], "error");
        assert!(
            value["msg"]
                .as_str()
                .unwrap()
                .contains("unclosed quote service=api")
        );
    }

    #[test]
    fn test_csv_line_fields() {
        let record = parse_csv_line(r#"John,30,"Engineer, Senior""#).unwrap();
        assert_eq!(record.get(0), Some("John"));
        assert_eq!(record.get(1), Some("30"));
        assert_eq!(record.get(2), Some("Engineer, Senior"));
    }
}
