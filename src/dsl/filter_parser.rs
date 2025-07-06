//! Filter expression parser

use pest::iterators::Pair;

use super::grammar::{DSLParser, Rule};
use crate::filter::{ComparisonOp, FieldPath, FilterExpr, FilterValue};

pub struct FilterParser;

impl FilterParser {
    pub fn parse_filter_expr(
        pair: Pair<Rule>,
    ) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let inner = pair.into_inner().next().unwrap();
        Self::parse_condition(inner)
    }

    fn parse_condition(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let inner = pair.into_inner().next().unwrap();
        Self::parse_or_expr(inner)
    }

    fn parse_or_expr(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let mut left = Self::parse_and_expr(inner.next().ok_or_else(|| {
            pest::error::Error::new_from_pos(
                pest::error::ErrorVariant::CustomError {
                    message: "Expected first expression in OR".to_string(),
                },
                span.start_pos(),
            )
        })?)?;

        while let Some(op_pair) = inner.next() {
            if matches!(op_pair.as_rule(), Rule::or_op) {
                let right = Self::parse_and_expr(inner.next().ok_or_else(|| {
                    pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: "Expected second expression after OR".to_string(),
                        },
                        span.start_pos(),
                    )
                })?)?;
                left = FilterExpr::Or(Box::new(left), Box::new(right));
            }
        }

        Ok(left)
    }

    fn parse_and_expr(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let mut left = match inner.next() {
            Some(first) => Self::parse_not_expr(first)?,
            None => {
                return Err(Box::new(pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: "Expected expression in AND".to_string(),
                    },
                    span.start_pos(),
                )))
            }
        };

        while let Some(op_pair) = inner.next() {
            if matches!(op_pair.as_rule(), Rule::and_op) {
                let op_span = op_pair.as_span();
                let right = match inner.next() {
                    Some(expr) => Self::parse_not_expr(expr)?,
                    None => {
                        return Err(Box::new(pest::error::Error::new_from_pos(
                            pest::error::ErrorVariant::CustomError {
                                message: "Expected expression after AND".to_string(),
                            },
                            op_span.end_pos(),
                        )))
                    }
                };
                left = FilterExpr::And(Box::new(left), Box::new(right));
            }
        }

        Ok(left)
    }

    fn parse_not_expr(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let mut inner = pair.into_inner();
        let first = inner.next().unwrap();

        match first.as_rule() {
            Rule::not_op => {
                // Handle negation
                if let Some(next) = inner.next() {
                    let expr = Self::parse_not_expr(next)?;
                    Ok(FilterExpr::Not(Box::new(expr)))
                } else {
                    Err(Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: "Expected expression after NOT".to_string(),
                        },
                        first.as_span().end_pos(),
                    )))
                }
            }
            Rule::comparison_expr => {
                // Handle comparison expression
                Self::parse_comparison_expr(first)
            }
            _ => Err(Box::new(pest::error::Error::new_from_pos(
                pest::error::ErrorVariant::CustomError {
                    message: format!("Unexpected rule in not_expr: {:?}", first.as_rule()),
                },
                first.as_span().start_pos(),
            ))),
        }
    }

    fn parse_comparison_expr(
        pair: Pair<Rule>,
    ) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let first = match inner.next() {
            Some(first) => first,
            None => {
                return Err(Box::new(pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: "Expected expression in comparison".to_string(),
                    },
                    span.start_pos(),
                )))
            }
        };

        match first.as_rule() {
            Rule::field_path => {
                let field_span = first.as_span();
                let field = DSLParser::parse_field_path(first);
                if let Some(op_pair) = inner.next() {
                    let op = Self::parse_comparison_op(op_pair);
                    if let Some(value_pair) = inner.next() {
                        let value = Self::parse_value(value_pair);
                        Ok(FilterExpr::Comparison { field, op, value })
                    } else {
                        Err(Box::new(pest::error::Error::new_from_pos(
                            pest::error::ErrorVariant::CustomError {
                                message: "Expected value after comparison operator".to_string(),
                            },
                            field_span.end_pos(),
                        )))
                    }
                } else {
                    // This case handles bare field paths that appear in comparison_expr
                    // According to the grammar, this shouldn't happen in boolean context
                    // but might occur during fallback parsing
                    Err(Box::new(pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: "Bare field in expression - use 'field?' for truthy check or add comparison operator".to_string(),
                    },
                    field_span.start_pos(),
                )))
                }
            }
            Rule::field_truthy => Self::parse_field_truthy(first),
            Rule::boolean_expr => {
                // Handle parenthesized boolean expression
                Self::parse_condition(first)
            }
            _ => Err(Box::new(pest::error::Error::new_from_pos(
                pest::error::ErrorVariant::CustomError {
                    message: format!("Unexpected rule in comparison_expr: {:?}", first.as_rule()),
                },
                first.as_span().start_pos(),
            ))),
        }
    }

    fn parse_field_truthy(first: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        // Handle field? syntax for explicit truthy checks
        let field_str = first.as_str();
        let span = first.as_span();
        let inner_pairs: Vec<_> = first.into_inner().collect();
        if let Some(field_pair) = inner_pairs.first() {
            let field = DSLParser::parse_field_path(field_pair.clone());
            Ok(FilterExpr::FieldTruthy(field))
        } else {
            // field_truthy is atomic in the grammar, parse it directly
            if let Some(field_name) = field_str.strip_suffix('?') {
                let parts: Vec<String> = field_name.split('.').map(|s| s.to_string()).collect();
                let field = FieldPath::new(parts);
                Ok(FilterExpr::FieldTruthy(field))
            } else {
                Err(Box::new(pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: "Invalid field truthy syntax".to_string(),
                    },
                    span.start_pos(),
                )))
            }
        }
    }

    fn parse_comparison_op(pair: Pair<Rule>) -> ComparisonOp {
        super::operators::parse_comparison_op(pair.as_str())
    }

    fn parse_value(pair: Pair<Rule>) -> FilterValue {
        let pair_str = pair.as_str().to_string(); // Clone the string first
        let inner = match pair.into_inner().next() {
            Some(inner) => inner,
            None => {
                // Fallback - treat the whole pair as a string value
                return FilterValue::String(pair_str);
            }
        };
        match inner.as_rule() {
            Rule::string_literal => {
                let string_content = inner.into_inner().next().unwrap();
                let content = string_content.as_str().to_string();
                FilterValue::String(content)
            }
            Rule::regex_literal => {
                // For now, treat regex literals as strings (simple contains matching)
                // Extract the pattern between the / / delimiters
                let regex_str = inner.as_str();
                if regex_str.starts_with('/') && regex_str.ends_with('/') {
                    let pattern = &regex_str[1..regex_str.len() - 1];
                    FilterValue::String(pattern.to_string())
                } else {
                    FilterValue::String(regex_str.to_string())
                }
            }
            Rule::number => {
                let number_str = inner.as_str();
                let num: f64 = number_str.parse().unwrap_or(0.0);
                FilterValue::Number(num)
            }
            Rule::boolean => {
                let bool_val = inner.as_str() == "true";
                FilterValue::Boolean(bool_val)
            }
            Rule::null => FilterValue::Null,
            Rule::field_path => FilterValue::String(inner.as_str().to_string()),
            _ => FilterValue::String(inner.as_str().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::grammar::DSLParser;
    use super::*;
    use crate::filter::{ComparisonOp, FilterExpr, FilterValue};

    fn parse_filter_string(input: &str) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        DSLParser::parse_filter_only(input)
    }

    #[test]
    fn test_parse_filter_expr() {
        let result = parse_filter_string("age > 25").unwrap();
        match result {
            FilterExpr::Comparison { field, op, value } => {
                assert_eq!(field.parts, vec!["age"]);
                assert!(matches!(op, ComparisonOp::GreaterThan));
                assert!(matches!(value, FilterValue::Number(25.0)));
            }
            _ => panic!("Expected comparison"),
        }
    }

    #[test]
    fn test_parse_boolean_and() {
        let result = parse_filter_string("active? && verified?").unwrap();
        match result {
            FilterExpr::And(left, right) => match (left.as_ref(), right.as_ref()) {
                (FilterExpr::FieldTruthy(l), FilterExpr::FieldTruthy(r)) => {
                    assert_eq!(l.parts, vec!["active"]);
                    assert_eq!(r.parts, vec!["verified"]);
                }
                _ => panic!("Expected FieldTruthy expressions"),
            },
            _ => panic!("Expected AND expression"),
        }
    }

    #[test]
    fn test_parse_boolean_or() {
        let result = parse_filter_string("premium? || admin?").unwrap();
        match result {
            FilterExpr::Or(left, right) => match (left.as_ref(), right.as_ref()) {
                (FilterExpr::FieldTruthy(l), FilterExpr::FieldTruthy(r)) => {
                    assert_eq!(l.parts, vec!["premium"]);
                    assert_eq!(r.parts, vec!["admin"]);
                }
                _ => panic!("Expected FieldTruthy expressions"),
            },
            _ => panic!("Expected OR expression"),
        }
    }

    #[test]
    fn test_parse_not_expression() {
        let result = parse_filter_string("!suspended?").unwrap();
        match result {
            FilterExpr::Not(inner) => match inner.as_ref() {
                FilterExpr::FieldTruthy(field) => {
                    assert_eq!(field.parts, vec!["suspended"]);
                }
                _ => panic!("Expected FieldTruthy inside NOT"),
            },
            _ => panic!("Expected NOT expression"),
        }
    }

    #[test]
    fn test_parse_field_truthy() {
        let result = parse_filter_string("active?").unwrap();
        match result {
            FilterExpr::FieldTruthy(field) => {
                assert_eq!(field.parts, vec!["active"]);
            }
            _ => panic!("Expected FieldTruthy"),
        }
    }

    #[test]
    fn test_parse_in_expression() {
        // The 'in' operator has been removed from the grammar
        // This test should now expect a parse error
        let result = parse_filter_string("status in [\"active\", \"pending\"]");
        assert!(result.is_err(), "IN operator should no longer be supported");
    }

    #[test]
    fn test_comparison_operators() {
        let test_cases = vec![
            ("age == 25", ComparisonOp::Equal),
            ("age != 25", ComparisonOp::NotEqual),
            ("age > 25", ComparisonOp::GreaterThan),
            ("age >= 25", ComparisonOp::GreaterThanOrEqual),
            ("age < 25", ComparisonOp::LessThan),
            ("age <= 25", ComparisonOp::LessThanOrEqual),
            ("name ~= /pattern/", ComparisonOp::Matches),
            ("name *= \"text\"", ComparisonOp::Contains),
            ("name ^= \"prefix\"", ComparisonOp::StartsWith),
            ("name $= \"suffix\"", ComparisonOp::EndsWith),
        ];

        for (input, expected_op) in test_cases {
            let result = parse_filter_string(input).unwrap();
            match result {
                FilterExpr::Comparison { op, .. } => {
                    assert!(std::mem::discriminant(&op) == std::mem::discriminant(&expected_op));
                }
                _ => panic!("Expected comparison for: {input}"),
            }
        }
    }

    #[test]
    fn test_complex_expressions() {
        // Parenthesized expressions
        let result = parse_filter_string("(age > 18 && age < 65) || retired?").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));

        // Mixed comparisons and truthy
        let result = parse_filter_string("active? && score >= 80").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));

        // Nested NOT
        let result = parse_filter_string("!(suspended? || banned?)").unwrap();
        assert!(matches!(result, FilterExpr::Not(_)));
    }

    #[test]
    fn test_value_parsing() {
        // String values
        let result = parse_filter_string("name == \"Alice\"").unwrap();
        match result {
            FilterExpr::Comparison {
                value: FilterValue::String(s),
                ..
            } => {
                assert_eq!(s, "Alice");
            }
            _ => panic!("Expected string value"),
        }

        // Number values
        let result = parse_filter_string("age == 25.5").unwrap();
        match result {
            FilterExpr::Comparison {
                value: FilterValue::Number(n),
                ..
            } => {
                assert_eq!(n, 25.5);
            }
            _ => panic!("Expected number value"),
        }

        // Boolean values
        let result = parse_filter_string("active == true").unwrap();
        match result {
            FilterExpr::Comparison {
                value: FilterValue::Boolean(b),
                ..
            } => {
                assert!(b);
            }
            _ => panic!("Expected boolean value"),
        }

        // Null values
        let result = parse_filter_string("data == null").unwrap();
        match result {
            FilterExpr::Comparison {
                value: FilterValue::Null,
                ..
            } => {}
            _ => panic!("Expected null value"),
        }
    }

    #[test]
    fn test_nested_field_paths() {
        let result = parse_filter_string("user.profile.email == \"test@example.com\"").unwrap();
        match result {
            FilterExpr::Comparison { field, .. } => {
                assert_eq!(field.parts, vec!["user", "profile", "email"]);
            }
            _ => panic!("Expected comparison with nested field"),
        }
    }

    #[test]
    fn test_regex_literals() {
        let result = parse_filter_string("name ~= /[A-Z][a-z]+/").unwrap();
        match result {
            FilterExpr::Comparison {
                value: FilterValue::String(pattern),
                ..
            } => {
                assert_eq!(pattern, "[A-Z][a-z]+");
            }
            _ => panic!("Expected regex pattern as string"),
        }
    }
}
