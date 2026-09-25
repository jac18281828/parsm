//! A record: one parsed value plus its own source text.
//!
//! Convert mode writes [`Record::to_json`]; filters, templates and field
//! selectors read [`Record::view`], which always carries `$0`.

use serde_json::{Map, Value};
use std::sync::Arc;

/// Key under which every view exposes the record's source text.
pub const SOURCE_KEY: &str = "$0";

/// A CSV header row: names as written and the keys the DSL matches.
#[derive(Debug)]
pub(crate) struct CsvHeader {
    names: Vec<String>,
    keys: Vec<String>,
}

impl CsvHeader {
    pub(crate) fn new(fields: &csv::StringRecord) -> Self {
        let names: Vec<String> = fields.iter().map(|f| f.trim().to_string()).collect();
        let keys = names.iter().map(|name| name.to_lowercase()).collect();
        Self { names, keys }
    }
}

#[derive(Debug)]
enum Content {
    /// JSON, YAML, TOML and logfmt.
    Value(Value),
    /// A CSV row.
    Row {
        fields: Vec<String>,
        header: Option<Arc<CsvHeader>>,
    },
    /// A text line split on whitespace.
    Words(Vec<String>),
}

/// One unit of input: a parsed value and the text it came from.
#[derive(Debug)]
pub struct Record {
    source: String,
    content: Content,
}

impl Record {
    /// A JSON, YAML, TOML or logfmt value.
    pub fn value(value: Value, source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            content: Content::Value(value),
        }
    }

    /// A CSV row, keyed by `header` when the input has one.
    pub(crate) fn row(
        fields: &csv::StringRecord,
        header: Option<Arc<CsvHeader>>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            content: Content::Row {
                fields: fields.iter().map(str::to_string).collect(),
                header,
            },
        }
    }

    /// A text line.
    pub fn text(line: impl Into<String>) -> Self {
        let source = line.into();
        let words = source.split_whitespace().map(str::to_string).collect();
        Self {
            source,
            content: Content::Words(words),
        }
    }

    /// The record's own source text, exposed as `$0`.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The record as convert mode writes it.
    pub fn to_json(&self) -> Value {
        match &self.content {
            Content::Value(value) => value.clone(),
            Content::Row {
                fields,
                header: Some(header),
            } => Value::Object(
                header
                    .names
                    .iter()
                    .zip(fields)
                    .map(|(name, field)| (name.clone(), Value::String(field.clone())))
                    .collect(),
            ),
            Content::Row { fields, .. } | Content::Words(fields) => strings(fields),
        }
    }

    /// The record as filters, templates and field selectors see it.
    pub fn view(&self) -> Value {
        let mut view = match &self.content {
            Content::Value(value) => value_view(value),
            Content::Row { fields, header } => row_view(fields, header.as_deref(), &self.source),
            Content::Words(words) => words_view(words),
        };
        view.insert(SOURCE_KEY.to_string(), Value::String(self.source.clone()));
        Value::Object(view)
    }
}

/// An object keeps its keys, an array exposes its indexes and a scalar
/// exposes only `$0`.
fn value_view(value: &Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map.clone(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item.clone()))
            .collect(),
        _ => Map::new(),
    }
}

/// `0` and `${0}` hold the line, `1`… and `field_0`… the fields, header
/// names (lower-cased, bare, `$name` and `${name}`) the fields under a
/// header, and `_array` every field.
fn row_view(fields: &[String], header: Option<&CsvHeader>, source: &str) -> Map<String, Value> {
    let mut view = Map::new();
    view.insert("0".to_string(), Value::String(source.to_string()));
    view.insert("${0}".to_string(), Value::String(source.to_string()));
    for (index, field) in fields.iter().enumerate() {
        let field = Value::String(field.clone());
        view.insert((index + 1).to_string(), field.clone());
        view.insert(format!("field_{index}"), field);
    }
    if let Some(header) = header {
        for (key, field) in header.keys.iter().zip(fields) {
            let field = Value::String(field.clone());
            view.insert(key.clone(), field.clone());
            view.insert(format!("${key}"), field.clone());
            view.insert(format!("${{{key}}}"), field);
        }
    }
    view.insert("_array".to_string(), strings(fields));
    view
}

/// `word_0`… hold the words and `_array` all of them.
fn words_view(words: &[String]) -> Map<String, Value> {
    let mut view = Map::new();
    for (index, word) in words.iter().enumerate() {
        view.insert(format!("word_{index}"), Value::String(word.clone()));
    }
    view.insert("_array".to_string(), strings(words));
    view
}

fn strings(items: &[String]) -> Value {
    Value::Array(items.iter().cloned().map(Value::String).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn csv_fields(line: &str) -> csv::StringRecord {
        crate::parse::parse_csv_line(line).unwrap()
    }

    #[test]
    fn object_view_adds_source() {
        let record = Record::value(json!({"a": 1}), r#"{"a": 1}"#);
        assert_eq!(record.view(), json!({"a": 1, "$0": r#"{"a": 1}"#}));
        assert_eq!(record.to_json(), json!({"a": 1}));
    }

    #[test]
    fn scalar_and_array_views_expose_source() {
        assert_eq!(Record::value(json!(42), "42").view(), json!({"$0": "42"}));
        let sequence = Record::value(json!(["a"]), "- a");
        assert_eq!(sequence.view(), json!({"0": "a", "$0": "- a"}));
        assert_eq!(sequence.to_json(), json!(["a"]));
    }

    #[test]
    fn csv_row_view_without_header() {
        let record = Record::row(&csv_fields("Alice,30"), None, "Alice,30");
        assert_eq!(
            record.view(),
            json!({
                "0": "Alice,30", "${0}": "Alice,30", "$0": "Alice,30",
                "1": "Alice", "field_0": "Alice", "2": "30", "field_1": "30",
                "_array": ["Alice", "30"]
            })
        );
        assert_eq!(record.to_json(), json!(["Alice", "30"]));
    }

    #[test]
    fn csv_row_with_header_keys_by_name() {
        let header = Arc::new(CsvHeader::new(&csv_fields("Name,age")));
        let record = Record::row(&csv_fields("Tom,45"), Some(header), "Tom,45");
        let view = record.view();
        assert_eq!(view["name"], "Tom");
        assert_eq!(view["$name"], "Tom");
        assert_eq!(view["${age}"], "45");
        assert_eq!(view["field_1"], "45");
        let converted = serde_json::to_string(&record.to_json()).unwrap();
        assert_eq!(converted, r#"{"Name":"Tom","age":"45"}"#);
    }

    #[test]
    fn text_view_splits_words() {
        let record = Record::text("hello  world");
        assert_eq!(
            record.view(),
            json!({
                "word_0": "hello", "word_1": "world",
                "_array": ["hello", "world"], "$0": "hello  world"
            })
        );
        assert_eq!(record.to_json(), json!(["hello", "world"]));
    }
}
