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
    Regex {
        field: FieldPath,
        pattern: String,
        flags: Option<String>,
    },
    In {
        field: FieldPath,
        values: Vec<FilterValue>,
    },
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
    Matches, // For regex matching
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
            FilterExpr::Regex {
                field,
                pattern,
                flags,
            } => Self::evaluate_regex(field, pattern, flags.as_deref(), data),
            FilterExpr::In { field, values } => Self::evaluate_in(field, values, data),
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
            ComparisonOp::Matches => Self::regex_matches(data_value, filter_value),
        }
    }

    fn evaluate_regex(field: &FieldPath, pattern: &str, flags: Option<&str>, data: &Value) -> bool {
        let data_value = match field.get_value(data) {
            Some(v) => v,
            None => return false,
        };

        let text = match data_value {
            Value::String(s) => s,
            _ => return false,
        };

        // Simple regex implementation for now - could be enhanced with actual regex crate
        if let Some(_flags) = flags {
            // For now, just do case-insensitive matching if 'i' flag is present
            if flags.unwrap_or("").contains('i') {
                return text.to_lowercase().contains(&pattern.to_lowercase());
            }
        }

        text.contains(pattern)
    }

    fn evaluate_in(field: &FieldPath, values: &[FilterValue], data: &Value) -> bool {
        let data_value = match field.get_value(data) {
            Some(v) => v,
            None => return false,
        };

        let field_value = FilterValue::from_json(data_value);

        // Check if the field value matches any of the values in the array
        values.iter().any(|v| Self::values_equal(&field_value, v))
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
        match (data_value, filter_value) {
            (Value::String(data), FilterValue::String(pattern)) => data.contains(pattern),
            _ => false,
        }
    }

    fn string_starts_with(data_value: &Value, filter_value: &FilterValue) -> bool {
        match (data_value, filter_value) {
            (Value::String(data), FilterValue::String(pattern)) => data.starts_with(pattern),
            _ => false,
        }
    }

    fn string_ends_with(data_value: &Value, filter_value: &FilterValue) -> bool {
        match (data_value, filter_value) {
            (Value::String(data), FilterValue::String(pattern)) => data.ends_with(pattern),
            _ => false,
        }
    }

    // The ~ operator is used for regex matching, a regex is expected on
    // the right-hand side, but for simplicity we will use substring matching
    fn regex_matches(data_value: &Value, filter_value: &FilterValue) -> bool {
        match (data_value, filter_value) {
            (Value::String(data), FilterValue::String(pattern)) => data.contains(pattern),
            (data_value, FilterValue::String(pattern)) => {
                let data_str = match data_value {
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    Value::Array(_) | Value::Object(_) => {
                        serde_json::to_string(data_value).unwrap_or_default()
                    }
                    Value::String(s) => s.clone(), // already handled above, but for completeness
                };
                data_str.contains(pattern)
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
}
