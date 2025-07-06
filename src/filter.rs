use serde_json::Value;
use tracing::{debug, trace};

#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpr {
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Comparison {
        field: FieldPath,
        op: ComparisonOp,
        value: FilterValue,
    },
    FieldTruthy(FieldPath),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPath {
    pub parts: Vec<String>,
}

impl FieldPath {
    pub fn new(parts: Vec<String>) -> Self {
        Self { parts }
    }

    pub fn single(name: String) -> Self {
        Self { parts: vec![name] }
    }

    // Navigate nested JSON/object structures and arrays
    pub fn get_value<'a>(&self, data: &'a Value) -> Option<&'a Value> {
        let mut current = data;
        for part in &self.parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                Value::Array(arr) => {
                    // Try to parse the part as an array index
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr.get(index)?;
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        Some(current)
    }

    /// Extract and format a field value from JSON data
    pub fn extract_field(&self, data: &Value) -> Option<String> {
        let value = self.get_value(data)?;

        // Format the extracted value as a simple string without JSON encoding
        match value {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Null => Some("null".to_string()),
            Value::Array(_) | Value::Object(_) => {
                // For complex types, use JSON representation
                serde_json::to_string_pretty(value).ok()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    Regex, // For ~= regex matching
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

impl FilterValue {
    pub fn from_json(value: &Value) -> Self {
        match value {
            Value::String(s) => FilterValue::String(s.clone()),
            Value::Number(n) => FilterValue::Number(n.as_f64().unwrap_or(0.0)),
            Value::Bool(b) => FilterValue::Boolean(*b),
            Value::Null => FilterValue::Null,
            _ => FilterValue::String(value.to_string()),
        }
    }
}

// Template system for output formatting
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub items: Vec<TemplateItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateItem {
    Field(FieldPath),
    Literal(String),
    Conditional {
        field: FieldPath,
        true_template: Template,
        false_template: Template,
    },
}

impl Template {
    pub fn render(&self, data: &Value) -> String {
        debug!("Template::render called with data: {:?}", data);
        let mut result = String::new();

        for item in &self.items {
            match item {
                TemplateItem::Field(field) => {
                    trace!("Template field: {:?}", field);
                    if let Some(value) = field.get_value(data) {
                        debug!("Field value found: {:?}", value);
                        result.push_str(&format_value(value));
                    } else {
                        debug!("Field value not found for: {:?}", field);
                    }
                }
                TemplateItem::Literal(text) => {
                    result.push_str(text);
                }
                TemplateItem::Conditional {
                    field,
                    true_template,
                    false_template,
                } => {
                    let is_truthy = FilterEngine::evaluate_field_truthiness(field, data);
                    let template_to_use = if is_truthy {
                        true_template
                    } else {
                        false_template
                    };
                    result.push_str(&template_to_use.render(data));
                }
            }
        }

        result
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

/// Convert a JSON value to a string for regex matching
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// Consolidated regex matching implementation
fn regex_match_string(text: &str, pattern: &str, flags: Option<&str>) -> bool {
    // Handle flags for regex compilation
    let case_insensitive = flags.is_some_and(|f| f.contains('i'));
    let multiline = flags.is_some_and(|f| f.contains('m'));
    let dot_matches_newline = flags.is_some_and(|f| f.contains('s'));

    let mut regex_builder = regex::RegexBuilder::new(pattern);
    regex_builder
        .case_insensitive(case_insensitive)
        .multi_line(multiline)
        .dot_matches_new_line(dot_matches_newline);

    match regex_builder.build() {
        Ok(regex) => regex.is_match(text),
        Err(_) => {
            // Fallback to substring matching if regex compilation fails
            if case_insensitive {
                text.to_lowercase().contains(&pattern.to_lowercase())
            } else {
                text.contains(pattern)
            }
        }
    }
}

// Filter evaluation engine
pub struct FilterEngine;

impl FilterEngine {
    pub fn evaluate(expr: &FilterExpr, data: &Value) -> bool {
        match expr {
            FilterExpr::And(left, right) => {
                Self::evaluate(left, data) && Self::evaluate(right, data)
            }
            FilterExpr::Or(left, right) => {
                Self::evaluate(left, data) || Self::evaluate(right, data)
            }
            FilterExpr::Not(inner) => !Self::evaluate(inner, data),
            FilterExpr::Comparison { field, op, value } => {
                Self::evaluate_comparison(field, op, value, data)
            }
            FilterExpr::FieldTruthy(field) => Self::evaluate_field_truthiness(field, data),
        }
    }

    pub fn evaluate_field_truthiness(field: &FieldPath, data: &Value) -> bool {
        match field.get_value(data) {
            Some(value) => match value {
                Value::Null => false,
                Value::Bool(b) => *b,
                Value::String(s) => {
                    // Handle string representations of boolean values
                    match s.to_lowercase().as_str() {
                        "false" | "f" | "0" | "no" | "off" => false,
                        "true" | "t" | "1" | "yes" | "on" => true,
                        _ => !s.is_empty(), // Non-empty non-boolean strings are truthy
                    }
                }
                Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
                Value::Array(arr) => !arr.is_empty(),
                Value::Object(obj) => !obj.is_empty(),
            },
            None => false, // Field doesn't exist, so it's falsy
        }
    }

    fn evaluate_comparison(
        field: &FieldPath,
        op: &ComparisonOp,
        filter_value: &FilterValue,
        data: &Value,
    ) -> bool {
        let data_value = match field.get_value(data) {
            Some(v) => v,
            None => return false, // Field doesn't exist
        };

        match op {
            ComparisonOp::Equal => {
                let data_filter_value = FilterValue::from_json(data_value);
                Self::values_equal(&data_filter_value, filter_value)
            }
            ComparisonOp::NotEqual => {
                let data_filter_value = FilterValue::from_json(data_value);
                !Self::values_equal(&data_filter_value, filter_value)
            }
            ComparisonOp::LessThan => Self::compare_numbers(data_value, filter_value, |a, b| a < b),
            ComparisonOp::LessThanOrEqual => {
                Self::compare_numbers(data_value, filter_value, |a, b| a <= b)
            }
            ComparisonOp::GreaterThan => {
                Self::compare_numbers(data_value, filter_value, |a, b| a > b)
            }
            ComparisonOp::GreaterThanOrEqual => {
                Self::compare_numbers(data_value, filter_value, |a, b| a >= b)
            }
            ComparisonOp::Contains => Self::string_contains(data_value, filter_value),
            ComparisonOp::StartsWith => Self::string_starts_with(data_value, filter_value),
            ComparisonOp::EndsWith => Self::string_ends_with(data_value, filter_value),
            ComparisonOp::Regex => {
                let pattern = match filter_value {
                    FilterValue::String(s) => s,
                    _ => return false,
                };
                let text = value_to_string(data_value);
                regex_match_string(&text, pattern, None)
            }
        }
    }

    fn values_equal(a: &FilterValue, b: &FilterValue) -> bool {
        match (a, b) {
            (FilterValue::String(a), FilterValue::String(b)) => a == b,
            (FilterValue::Number(a), FilterValue::Number(b)) => (a - b).abs() < f64::EPSILON,
            (FilterValue::Boolean(a), FilterValue::Boolean(b)) => a == b,
            (FilterValue::Null, FilterValue::Null) => true,
            // Handle cross-type comparisons: string vs number
            (FilterValue::String(s), FilterValue::Number(n)) => {
                if let Ok(parsed) = s.parse::<f64>() {
                    (parsed - n).abs() < f64::EPSILON
                } else {
                    false
                }
            }
            (FilterValue::Number(n), FilterValue::String(s)) => {
                if let Ok(parsed) = s.parse::<f64>() {
                    (n - parsed).abs() < f64::EPSILON
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn compare_numbers<F>(data_value: &Value, filter_value: &FilterValue, op: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        let data_num = match data_value {
            Value::Number(n) => Some(n.as_f64().unwrap_or(0.0)),
            Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        };

        let filter_num = match filter_value {
            FilterValue::Number(n) => Some(*n),
            FilterValue::String(s) => s.parse::<f64>().ok(),
            _ => None,
        };

        match (data_num, filter_num) {
            (Some(a), Some(b)) => op(a, b),
            _ => false,
        }
    }

    fn string_contains(data_value: &Value, filter_value: &FilterValue) -> bool {
        match filter_value {
            FilterValue::String(pattern) => {
                let text = value_to_string(data_value);
                text.contains(pattern)
            }
            _ => false,
        }
    }

    fn string_starts_with(data_value: &Value, filter_value: &FilterValue) -> bool {
        match filter_value {
            FilterValue::String(pattern) => {
                let text = value_to_string(data_value);
                text.starts_with(pattern)
            }
            _ => false,
        }
    }

    fn string_ends_with(data_value: &Value, filter_value: &FilterValue) -> bool {
        match filter_value {
            FilterValue::String(pattern) => {
                let text = value_to_string(data_value);
                text.ends_with(pattern)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_field_path_access() {
        let data = json!({
            "name": "Alice",
            "user": {
                "id": 123,
                "email": "alice@example.com"
            }
        });

        let name_path = FieldPath::single("name".to_string());
        assert_eq!(name_path.get_value(&data), Some(&json!("Alice")));

        let email_path = FieldPath::new(vec!["user".to_string(), "email".to_string()]);
        assert_eq!(
            email_path.get_value(&data),
            Some(&json!("alice@example.com"))
        );

        let missing_path = FieldPath::single("missing".to_string());
        assert_eq!(missing_path.get_value(&data), None);
    }

    #[test]
    fn test_simple_comparison() {
        let data = json!({"name": "Alice", "age": 30});

        let expr = FilterExpr::Comparison {
            field: FieldPath::single("name".to_string()),
            op: ComparisonOp::Equal,
            value: FilterValue::String("Alice".to_string()),
        };

        assert!(FilterEngine::evaluate(&expr, &data));

        let expr2 = FilterExpr::Comparison {
            field: FieldPath::single("age".to_string()),
            op: ComparisonOp::GreaterThan,
            value: FilterValue::Number(25.0),
        };

        assert!(FilterEngine::evaluate(&expr2, &data));
    }

    #[test]
    fn test_and_or_logic() {
        let data = json!({"name": "Alice", "age": 30});

        let name_check = FilterExpr::Comparison {
            field: FieldPath::single("name".to_string()),
            op: ComparisonOp::Equal,
            value: FilterValue::String("Alice".to_string()),
        };

        let age_check = FilterExpr::Comparison {
            field: FieldPath::single("age".to_string()),
            op: ComparisonOp::GreaterThan,
            value: FilterValue::Number(25.0),
        };

        let and_expr = FilterExpr::And(Box::new(name_check), Box::new(age_check));
        assert!(FilterEngine::evaluate(&and_expr, &data));
    }

    #[test]
    fn test_template_rendering() {
        let data = json!({"name": "Alice", "age": 30});

        let template = Template {
            items: vec![
                TemplateItem::Field(FieldPath::single("name".to_string())),
                TemplateItem::Literal(" is ".to_string()),
                TemplateItem::Field(FieldPath::single("age".to_string())),
                TemplateItem::Literal(" years old".to_string()),
            ],
        };

        let result = template.render(&data);
        assert_eq!(result, "Alice is 30 years old");
    }

    #[test]
    fn test_value_to_string() {
        assert_eq!(value_to_string(&json!("hello")), "hello");
        assert_eq!(value_to_string(&json!(42)), "42");
        assert_eq!(value_to_string(&json!(42.5)), "42.5");
        assert_eq!(value_to_string(&json!(true)), "true");
        assert_eq!(value_to_string(&json!(false)), "false");
        assert_eq!(value_to_string(&json!(null)), "null");

        // Test complex types - arrays and objects get JSON stringified
        let array_result = value_to_string(&json!([1, 2, 3]));
        assert!(
            array_result.contains("1") && array_result.contains("2") && array_result.contains("3")
        );

        let object_result = value_to_string(&json!({"key": "value"}));
        assert!(object_result.contains("key") && object_result.contains("value"));
    }

    #[test]
    fn test_regex_match_string() {
        // Basic pattern matching
        assert!(regex_match_string("hello world", "world", None));
        assert!(!regex_match_string("hello world", "xyz", None));

        // Case sensitivity without flags
        assert!(!regex_match_string("Hello World", "hello", None));

        // Case insensitive with 'i' flag
        assert!(regex_match_string("Hello World", "hello", Some("i")));
        assert!(regex_match_string("HELLO WORLD", "hello", Some("i")));

        // Regex patterns
        assert!(regex_match_string("test123", r"\d+", None));
        assert!(regex_match_string("user@example.com", r"@.*\.com", None));
        assert!(!regex_match_string("notanemail", r"@.*\.com", None));

        // Multiline flag
        assert!(regex_match_string("line1\nline2", "^line2", Some("m")));

        // Dot matches newline flag
        assert!(regex_match_string(
            "line1\nline2",
            "line1.*line2",
            Some("s")
        ));

        // Invalid regex should fall back to substring matching
        assert!(regex_match_string("test[bracket", "[bracket", None));
        assert!(regex_match_string("TEST[BRACKET", "[bracket", Some("i")));
    }

    #[test]
    fn test_filter_value_from_json() {
        assert_eq!(
            FilterValue::from_json(&json!("test")),
            FilterValue::String("test".to_string())
        );
        assert_eq!(
            FilterValue::from_json(&json!(42)),
            FilterValue::Number(42.0)
        );
        assert_eq!(
            FilterValue::from_json(&json!(42.5)),
            FilterValue::Number(42.5)
        );
        assert_eq!(
            FilterValue::from_json(&json!(true)),
            FilterValue::Boolean(true)
        );
        assert_eq!(
            FilterValue::from_json(&json!(false)),
            FilterValue::Boolean(false)
        );
        assert_eq!(FilterValue::from_json(&json!(null)), FilterValue::Null);

        // Complex types should be converted to string
        let complex_result = FilterValue::from_json(&json!([1, 2, 3]));
        if let FilterValue::String(s) = complex_result {
            assert!(s.contains("1") && s.contains("2") && s.contains("3"));
        } else {
            panic!("Expected string conversion for array");
        }
    }

    #[test]
    fn test_field_path_array_access() {
        let data = json!({
            "users": [
                {"name": "Alice", "id": 1},
                {"name": "Bob", "id": 2}
            ]
        });

        // Access array element by index
        let path = FieldPath::new(vec![
            "users".to_string(),
            "0".to_string(),
            "name".to_string(),
        ]);
        assert_eq!(path.get_value(&data), Some(&json!("Alice")));

        let path2 = FieldPath::new(vec!["users".to_string(), "1".to_string(), "id".to_string()]);
        assert_eq!(path2.get_value(&data), Some(&json!(2)));

        // Invalid index should return None
        let path3 = FieldPath::new(vec![
            "users".to_string(),
            "5".to_string(),
            "name".to_string(),
        ]);
        assert_eq!(path3.get_value(&data), None);

        // Non-numeric index on array should return None
        let path4 = FieldPath::new(vec!["users".to_string(), "invalid".to_string()]);
        assert_eq!(path4.get_value(&data), None);
    }

    #[test]
    fn test_field_extract() {
        let data = json!({
            "name": "Alice",
            "age": 30,
            "active": true,
            "metadata": null,
            "scores": [95, 87, 92],
            "profile": {"bio": "Software engineer"}
        });

        let name_path = FieldPath::single("name".to_string());
        assert_eq!(name_path.extract_field(&data), Some("Alice".to_string()));

        let age_path = FieldPath::single("age".to_string());
        assert_eq!(age_path.extract_field(&data), Some("30".to_string()));

        let active_path = FieldPath::single("active".to_string());
        assert_eq!(active_path.extract_field(&data), Some("true".to_string()));

        let null_path = FieldPath::single("metadata".to_string());
        assert_eq!(null_path.extract_field(&data), Some("null".to_string()));

        // Complex types should return pretty JSON
        let scores_path = FieldPath::single("scores".to_string());
        let scores_result = scores_path.extract_field(&data);
        assert!(scores_result.is_some());
        let scores_str = scores_result.unwrap();
        assert!(
            scores_str.contains("95") && scores_str.contains("87") && scores_str.contains("92")
        );

        // Missing field should return None
        let missing_path = FieldPath::single("missing".to_string());
        assert_eq!(missing_path.extract_field(&data), None);
    }

    #[test]
    fn test_string_operations() {
        let data = json!({"text": "Hello World", "number": 42});

        // Contains
        let contains_expr = FilterExpr::Comparison {
            field: FieldPath::single("text".to_string()),
            op: ComparisonOp::Contains,
            value: FilterValue::String("World".to_string()),
        };
        assert!(FilterEngine::evaluate(&contains_expr, &data));

        let not_contains_expr = FilterExpr::Comparison {
            field: FieldPath::single("text".to_string()),
            op: ComparisonOp::Contains,
            value: FilterValue::String("xyz".to_string()),
        };
        assert!(!FilterEngine::evaluate(&not_contains_expr, &data));

        // StartsWith
        let starts_with_expr = FilterExpr::Comparison {
            field: FieldPath::single("text".to_string()),
            op: ComparisonOp::StartsWith,
            value: FilterValue::String("Hello".to_string()),
        };
        assert!(FilterEngine::evaluate(&starts_with_expr, &data));

        // EndsWith
        let ends_with_expr = FilterExpr::Comparison {
            field: FieldPath::single("text".to_string()),
            op: ComparisonOp::EndsWith,
            value: FilterValue::String("World".to_string()),
        };
        assert!(FilterEngine::evaluate(&ends_with_expr, &data));

        // Test with non-string values (should convert to string)
        let number_contains_expr = FilterExpr::Comparison {
            field: FieldPath::single("number".to_string()),
            op: ComparisonOp::Contains,
            value: FilterValue::String("4".to_string()),
        };
        assert!(FilterEngine::evaluate(&number_contains_expr, &data));
    }

    #[test]
    fn test_regex_matching() {
        let data = json!({
            "email": "user@example.com",
            "phone": "123-456-7890",
            "text": "Hello World"
        });

        // Test regex matching with Comparison expression using Regex operator
        let email_regex = FilterExpr::Comparison {
            field: FieldPath::single("email".to_string()),
            op: ComparisonOp::Regex,
            value: FilterValue::String(r".*@.*\.com".to_string()),
        };
        assert!(FilterEngine::evaluate(&email_regex, &data));

        let phone_regex = FilterExpr::Comparison {
            field: FieldPath::single("phone".to_string()),
            op: ComparisonOp::Regex,
            value: FilterValue::String(r"\d{3}-\d{3}-\d{4}".to_string()),
        };
        assert!(FilterEngine::evaluate(&phone_regex, &data));

        // Test regex with case insensitive pattern
        let case_insensitive_regex = FilterExpr::Comparison {
            field: FieldPath::single("text".to_string()),
            op: ComparisonOp::Regex,
            value: FilterValue::String("(?i)hello".to_string()),
        };
        assert!(FilterEngine::evaluate(&case_insensitive_regex, &data));

        // Test regex that doesn't match
        let no_match_regex = FilterExpr::Comparison {
            field: FieldPath::single("text".to_string()),
            op: ComparisonOp::Regex,
            value: FilterValue::String(r"\d+".to_string()),
        };
        assert!(!FilterEngine::evaluate(&no_match_regex, &data));
    }

    #[test]
    fn test_field_truthiness() {
        let data = json!({
            "null_field": null,
            "false_bool": false,
            "true_bool": true,
            "empty_string": "",
            "nonempty_string": "hello",
            "zero_number": 0,
            "nonzero_number": 42,
            "empty_array": [],
            "nonempty_array": [1, 2, 3],
            "empty_object": {},
            "nonempty_object": {"key": "value"},
            "false_string": "false",
            "true_string": "true",
            "yes_string": "yes",
            "no_string": "no"
        });

        // Test various falsy values
        assert!(!FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("null_field".to_string()),
            &data
        ));
        assert!(!FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("false_bool".to_string()),
            &data
        ));
        assert!(!FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("empty_string".to_string()),
            &data
        ));
        assert!(!FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("zero_number".to_string()),
            &data
        ));
        assert!(!FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("empty_array".to_string()),
            &data
        ));
        assert!(!FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("empty_object".to_string()),
            &data
        ));
        assert!(!FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("false_string".to_string()),
            &data
        ));
        assert!(!FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("no_string".to_string()),
            &data
        ));

        // Test various truthy values
        assert!(FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("true_bool".to_string()),
            &data
        ));
        assert!(FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("nonempty_string".to_string()),
            &data
        ));
        assert!(FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("nonzero_number".to_string()),
            &data
        ));
        assert!(FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("nonempty_array".to_string()),
            &data
        ));
        assert!(FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("nonempty_object".to_string()),
            &data
        ));
        assert!(FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("true_string".to_string()),
            &data
        ));
        assert!(FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("yes_string".to_string()),
            &data
        ));

        // Test missing field
        assert!(!FilterEngine::evaluate_field_truthiness(
            &FieldPath::single("missing_field".to_string()),
            &data
        ));
    }

    #[test]
    fn test_numeric_comparisons() {
        let data = json!({
            "age": 30,
            "score": 85.5,
            "string_number": "42",
            "non_number": "hello"
        });

        // Less than
        let lt_expr = FilterExpr::Comparison {
            field: FieldPath::single("age".to_string()),
            op: ComparisonOp::LessThan,
            value: FilterValue::Number(35.0),
        };
        assert!(FilterEngine::evaluate(&lt_expr, &data));

        // Greater than or equal
        let gte_expr = FilterExpr::Comparison {
            field: FieldPath::single("score".to_string()),
            op: ComparisonOp::GreaterThanOrEqual,
            value: FilterValue::Number(85.5),
        };
        assert!(FilterEngine::evaluate(&gte_expr, &data));

        // String to number comparison
        let string_num_expr = FilterExpr::Comparison {
            field: FieldPath::single("string_number".to_string()),
            op: ComparisonOp::Equal,
            value: FilterValue::Number(42.0),
        };
        assert!(FilterEngine::evaluate(&string_num_expr, &data));

        // Non-number string should fail numeric comparison
        let non_num_expr = FilterExpr::Comparison {
            field: FieldPath::single("non_number".to_string()),
            op: ComparisonOp::GreaterThan,
            value: FilterValue::Number(10.0),
        };
        assert!(!FilterEngine::evaluate(&non_num_expr, &data));
    }

    #[test]
    fn test_template_conditionals() {
        let data = json!({
            "has_admin": true,
            "user_count": 0,
            "name": "Alice"
        });

        let conditional_template = Template {
            items: vec![
                TemplateItem::Literal("User: ".to_string()),
                TemplateItem::Field(FieldPath::single("name".to_string())),
                TemplateItem::Conditional {
                    field: FieldPath::single("has_admin".to_string()),
                    true_template: Template {
                        items: vec![TemplateItem::Literal(" (Admin)".to_string())],
                    },
                    false_template: Template {
                        items: vec![TemplateItem::Literal(" (Regular)".to_string())],
                    },
                },
            ],
        };

        let result = conditional_template.render(&data);
        assert_eq!(result, "User: Alice (Admin)");

        // Test with falsy condition
        let data2 = json!({
            "has_admin": false,
            "name": "Bob"
        });

        let result2 = conditional_template.render(&data2);
        assert_eq!(result2, "User: Bob (Regular)");
    }

    #[test]
    fn test_not_and_or_expressions() {
        let data = json!({"age": 25, "name": "Alice", "active": true});

        // Test NOT expression
        let not_expr = FilterExpr::Not(Box::new(FilterExpr::Comparison {
            field: FieldPath::single("active".to_string()),
            op: ComparisonOp::Equal,
            value: FilterValue::Boolean(false),
        }));
        assert!(FilterEngine::evaluate(&not_expr, &data));

        // Test OR expression
        let or_expr = FilterExpr::Or(
            Box::new(FilterExpr::Comparison {
                field: FieldPath::single("age".to_string()),
                op: ComparisonOp::LessThan,
                value: FilterValue::Number(20.0),
            }),
            Box::new(FilterExpr::Comparison {
                field: FieldPath::single("name".to_string()),
                op: ComparisonOp::Equal,
                value: FilterValue::String("Alice".to_string()),
            }),
        );
        assert!(FilterEngine::evaluate(&or_expr, &data)); // Should be true because name is Alice

        // Test AND expression with one false condition
        let and_expr = FilterExpr::And(
            Box::new(FilterExpr::Comparison {
                field: FieldPath::single("age".to_string()),
                op: ComparisonOp::LessThan,
                value: FilterValue::Number(20.0), // This is false (25 is not < 20)
            }),
            Box::new(FilterExpr::Comparison {
                field: FieldPath::single("name".to_string()),
                op: ComparisonOp::Equal,
                value: FilterValue::String("Alice".to_string()), // This is true
            }),
        );
        assert!(!FilterEngine::evaluate(&and_expr, &data)); // Should be false because age condition fails
    }

    #[test]
    fn test_cross_type_equality() {
        let data = json!({
            "string_num": "42",
            "actual_num": 42,
            "string_bool": "true",
            "actual_bool": true
        });

        // String number should equal actual number
        let string_to_num = FilterExpr::Comparison {
            field: FieldPath::single("string_num".to_string()),
            op: ComparisonOp::Equal,
            value: FilterValue::Number(42.0),
        };
        assert!(FilterEngine::evaluate(&string_to_num, &data));

        // Actual number should equal string number
        let num_to_string = FilterExpr::Comparison {
            field: FieldPath::single("actual_num".to_string()),
            op: ComparisonOp::Equal,
            value: FilterValue::String("42".to_string()),
        };
        assert!(FilterEngine::evaluate(&num_to_string, &data));
    }

    #[test]
    fn test_missing_comparison_operators() {
        let data = json!({
            "name": "Alice",
            "age": 30,
            "score": 85.5,
            "active": true
        });

        // NotEqual tests
        let not_equal_string = FilterExpr::Comparison {
            field: FieldPath::single("name".to_string()),
            op: ComparisonOp::NotEqual,
            value: FilterValue::String("Bob".to_string()),
        };
        assert!(FilterEngine::evaluate(&not_equal_string, &data));

        let not_equal_number = FilterExpr::Comparison {
            field: FieldPath::single("age".to_string()),
            op: ComparisonOp::NotEqual,
            value: FilterValue::Number(25.0),
        };
        assert!(FilterEngine::evaluate(&not_equal_number, &data));

        let not_equal_bool = FilterExpr::Comparison {
            field: FieldPath::single("active".to_string()),
            op: ComparisonOp::NotEqual,
            value: FilterValue::Boolean(false),
        };
        assert!(FilterEngine::evaluate(&not_equal_bool, &data));

        // NotEqual should return false when values are equal
        let not_equal_false = FilterExpr::Comparison {
            field: FieldPath::single("name".to_string()),
            op: ComparisonOp::NotEqual,
            value: FilterValue::String("Alice".to_string()),
        };
        assert!(!FilterEngine::evaluate(&not_equal_false, &data));

        // LessThanOrEqual tests
        let lte_true = FilterExpr::Comparison {
            field: FieldPath::single("age".to_string()),
            op: ComparisonOp::LessThanOrEqual,
            value: FilterValue::Number(30.0), // Equal case
        };
        assert!(FilterEngine::evaluate(&lte_true, &data));

        let lte_true2 = FilterExpr::Comparison {
            field: FieldPath::single("age".to_string()),
            op: ComparisonOp::LessThanOrEqual,
            value: FilterValue::Number(35.0), // Less than case
        };
        assert!(FilterEngine::evaluate(&lte_true2, &data));

        let lte_false = FilterExpr::Comparison {
            field: FieldPath::single("age".to_string()),
            op: ComparisonOp::LessThanOrEqual,
            value: FilterValue::Number(25.0), // Greater than case
        };
        assert!(!FilterEngine::evaluate(&lte_false, &data));

        // LessThanOrEqual with decimal numbers
        let lte_decimal = FilterExpr::Comparison {
            field: FieldPath::single("score".to_string()),
            op: ComparisonOp::LessThanOrEqual,
            value: FilterValue::Number(85.5), // Equal with decimal
        };
        assert!(FilterEngine::evaluate(&lte_decimal, &data));
    }
}
