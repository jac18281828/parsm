//! Filter expression parser

use pest::iterators::Pair;

use super::grammar::{DSLParser, Rule};
use crate::filter::{ComparisonOp, CompiledRegex, FieldPath, FilterExpr, FilterValue};

pub struct FilterParser;

impl FilterParser {
    pub fn parse_filter_expr(
        pair: Pair<Rule>,
    ) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        // filter_expr = { boolean_expr }: always exactly one inner pair.
        let inner = pair.into_inner().next().unwrap();
        Self::parse_condition(inner)
    }

    fn parse_condition(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        // boolean_expr = { or_expr }: always exactly one inner pair.
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
                )));
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
                        )));
                    }
                };
                left = FilterExpr::And(Box::new(left), Box::new(right));
            }
        }

        Ok(left)
    }

    fn parse_not_expr(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        // not_expr = { not_op ~ WS* ~ not_expr | comparison_expr }: one of
        // the two alternatives always leaves at least one inner pair.
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
                )));
            }
        };

        match first.as_rule() {
            Rule::field_path => {
                // field_path ~ WS* ~ comparison_op ~ WS* ~ value: this alt of
                // comparison_expr only matches with both an operator and a
                // value present, so both pairs are always here.
                let field = DSLParser::parse_field_path(first);
                let op_pair = inner.next().unwrap();
                let op_str = op_pair.as_str().to_string();
                let op = Self::parse_comparison_op(op_pair)?;
                let value_pair = inner.next().unwrap();
                let value_span = value_pair.as_span();
                let value = Self::parse_value(value_pair)?;
                // `~` and `*=` both mean Contains, one meaning per operator;
                // a regex literal is only valid after `~=`, which already
                // matches regexes. Name whichever symbol the user typed.
                if op == ComparisonOp::Contains && matches!(value, FilterValue::Regex(_)) {
                    return Err(Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: format!(
                                "'{op_str}' is contains, not regex match - use '~=' for a regex pattern"
                            ),
                        },
                        value_span.start_pos(),
                    )));
                }
                // A `~=` value compiles once at parse time, whether it came
                // as a `/pattern/flags` literal (already CompiledRegex from
                // parse_value), a plain string, or a number/boolean literal
                // stringified the same way `*=`/`^=`/`$=` stringify their
                // right-hand side. An invalid pattern is a parse error
                // naming it, not a silent non-match at every record.
                let value = if op == ComparisonOp::Regex {
                    let literal_pattern = match &value {
                        FilterValue::String(pattern) => Some(pattern.clone()),
                        FilterValue::Number(n) => Some(n.to_string()),
                        FilterValue::Boolean(b) => Some(b.to_string()),
                        _ => None,
                    };
                    match literal_pattern {
                        Some(pattern) => {
                            let compiled = CompiledRegex::compile(&pattern, None).map_err(|e| {
                                Box::new(pest::error::Error::new_from_pos(
                                    pest::error::ErrorVariant::CustomError {
                                        message: format!("invalid regex pattern '{pattern}': {e}"),
                                    },
                                    value_span.start_pos(),
                                ))
                            })?;
                            FilterValue::Regex(compiled)
                        }
                        None => value,
                    }
                } else {
                    value
                };
                Ok(FilterExpr::Comparison { field, op, value })
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

    fn parse_comparison_op(
        pair: Pair<Rule>,
    ) -> Result<ComparisonOp, Box<pest::error::Error<Rule>>> {
        let span = pair.as_span();
        super::operators::parse_comparison_op(pair.as_str()).map_err(|message| {
            Box::new(pest::error::Error::new_from_pos(
                pest::error::ErrorVariant::CustomError { message },
                span.start_pos(),
            ))
        })
    }

    fn parse_value(pair: Pair<Rule>) -> Result<FilterValue, Box<pest::error::Error<Rule>>> {
        // value = { string_literal | regex_literal | number | boolean | null
        // | field_path }: exactly one alternative always matches.
        let span = pair.as_span();
        let inner = pair.into_inner().next().unwrap();
        Ok(match inner.as_rule() {
            Rule::string_literal => {
                // string_literal = { "\"" ~ string_content ~ "\"" | "'" ~
                // string_content_single ~ "'" }: always exactly one inner pair.
                let string_content = inner.into_inner().next().unwrap();
                let content = DSLParser::unescape_string_content(string_content.as_str());
                FilterValue::String(content)
            }
            Rule::regex_literal => {
                // Regex patterns must reach the engine byte-preserving (raw, not
                // unescaped) so e.g. `\d` stays `\d` - unescaping would corrupt
                // the pattern into `d`. Read the grammar's own regex_content /
                // regex_flags pairs rather than slicing the raw text, since the
                // latter breaks once optional flags follow the closing `/`.
                let mut regex_inner = inner.into_inner();
                let pattern = regex_inner
                    .next()
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                let flags = regex_inner.next().map(|p| p.as_str().to_string());
                let compiled = CompiledRegex::compile(&pattern, flags.as_deref()).map_err(|e| {
                    Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: format!("invalid regex pattern '{pattern}': {e}"),
                        },
                        span.start_pos(),
                    ))
                })?;
                FilterValue::Regex(compiled)
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
            Rule::field_path => FilterValue::FieldRef(DSLParser::parse_field_path(inner)),
            _ => FilterValue::String(inner.as_str().to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::grammar::DSLParser;
    use super::*;
    use crate::filter::{ComparisonOp, FilterExpr, FilterValue};

    fn parse_filter_string(input: &str) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        // The grammar's filter_expr is atomic per input here (no field
        // selector or template alternative can also match), so a filter is
        // always present on success.
        DSLParser::parse_dsl(input).map(|dsl| dsl.filter.unwrap())
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
                value: FilterValue::Regex(compiled),
                ..
            } => {
                assert_eq!(compiled.pattern, "[A-Z][a-z]+");
                assert_eq!(compiled.flags, None);
            }
            _ => panic!("Expected regex pattern"),
        }
    }

    #[test]
    fn test_regex_literal_with_flags() {
        let result = parse_filter_string("email ~= /alice/i").unwrap();
        match result {
            FilterExpr::Comparison {
                value: FilterValue::Regex(compiled),
                ..
            } => {
                assert_eq!(compiled.pattern, "alice");
                assert_eq!(compiled.flags.as_deref(), Some("i"));
            }
            _ => panic!("Expected regex pattern with flags"),
        }
    }

    #[test]
    fn invalid_regex_literal_is_a_parse_error_naming_the_pattern() {
        let err = parse_filter_string("name ~= /[b/").unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("[b"),
            "error should name the invalid pattern: {message}"
        );
    }

    #[test]
    fn regex_literal_honors_the_x_flag() {
        // The 'x' (extended) flag lets whitespace in the pattern be
        // insignificant, so "a b" matches "ab".
        let result = parse_filter_string("s ~= /a b/x").unwrap();
        match result {
            FilterExpr::Comparison {
                value: FilterValue::Regex(compiled),
                ..
            } => assert!(compiled.is_match("ab")),
            _ => panic!("Expected regex pattern"),
        }
    }

    #[test]
    fn tilde_contains_rejects_a_regex_literal() {
        let err = parse_filter_string("name ~ /A.*e/").unwrap_err();
        assert!(err.to_string().contains("~="));
    }
}
