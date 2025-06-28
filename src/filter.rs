use serde_json::Value;

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
}

impl Template {
    pub fn render(&self, data: &Value) -> String {
        let mut result = String::new();

        for item in &self.items {
            match item {
                TemplateItem::Field(field) => {
                    if let Some(value) = field.get_value(data) {
                        result.push_str(&format_value(value));
                    }
                }
                TemplateItem::Literal(text) => {
                    result.push_str(text);
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
        }
    }

    fn evaluate_field_truthiness(field: &FieldPath, data: &Value) -> bool {
        match field.get_value(data) {
            Some(value) => match value {
                Value::Null => false,
                Value::Bool(b) => *b,
                Value::String(s) => !s.is_empty(),
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
            ComparisonOp::Equal => Self::values_equal(data_value, filter_value),
            ComparisonOp::NotEqual => !Self::values_equal(data_value, filter_value),
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

    fn values_equal(data_value: &Value, filter_value: &FilterValue) -> bool {
        match (data_value, filter_value) {
            (Value::String(a), FilterValue::String(b)) => a == b,
            (Value::Number(a), FilterValue::Number(b)) => {
                (a.as_f64().unwrap_or(0.0) - b).abs() < f64::EPSILON
            }
            (Value::Bool(a), FilterValue::Boolean(b)) => a == b,
            (Value::Null, FilterValue::Null) => true,
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

    fn regex_matches(data_value: &Value, filter_value: &FilterValue) -> bool {
        match (data_value, filter_value) {
            (Value::String(data), FilterValue::String(pattern)) => data.contains(pattern),
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
