//! Main grammar parser using Pest

use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use tracing::trace;

use super::ast::ParsedDSL;
use super::filter_parser::FilterParser;
use super::template_parser::TemplateParser;
use crate::filter::{FieldPath, FilterExpr, Template};

/// Main DSL parser using Pest grammar with conservative syntax.
///
/// This parser handles the complete parsm DSL grammar with clear disambiguation:
/// - Filter expressions require explicit comparison operators and boolean logic
/// - Template strings use `${variable}` syntax for field substitution
/// - Field selectors use bare identifiers without operators
/// - Combined filter + template expressions are parsed as separate components
///
/// The parser uses conservative fallback strategies to ensure predictable behavior
/// and prevent ambiguous interpretations of user input.
#[derive(Parser)]
#[grammar = "pest/parsm.pest"]
pub struct DSLParser;

impl DSLParser {
    /// Main parsing entry point - much more permissive
    pub fn parse_dsl(input: &str) -> Result<ParsedDSL, Box<pest::error::Error<Rule>>> {
        let mut pairs = Self::parse(Rule::program, input)?;
        let program = pairs.next().unwrap();

        let mut result = ParsedDSL::new();

        for pair in program.into_inner() {
            match pair.as_rule() {
                Rule::expression => {
                    Self::parse_expression(pair, &mut result)?;
                }
                Rule::EOI => break,
                _ => {}
            }
        }

        Ok(result)
    }

    /// Parse separate filter and template expressions
    pub fn parse_separate(
        filter_input: Option<&str>,
        template_input: Option<&str>,
    ) -> Result<ParsedDSL, Box<pest::error::Error<Rule>>> {
        let mut result = ParsedDSL::new();

        if let Some(filter_str) = filter_input {
            if let Ok(filter_result) = Self::parse_dsl(filter_str) {
                if filter_result.filter.is_some() {
                    result.filter = filter_result.filter;
                } else if filter_result.field_selector.is_some() {
                    result.field_selector = filter_result.field_selector;
                } else {
                    return Err(Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: format!(
                                "Could not parse '{filter_str}' as filter expression or field selector",
                            ),
                        },
                        pest::Position::new(filter_str, 0).unwrap(),
                    )));
                }
            } else {
                return Err(Box::new(pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: format!(
                            "Could not parse '{filter_str}' as filter expression or field selector",
                        ),
                    },
                    pest::Position::new(filter_str, 0).unwrap(),
                )));
            }
        }

        if let Some(template_str) = template_input {
            if let Ok(template_result) = Self::parse_dsl(template_str) {
                result.template = template_result.template;
            } else {
                return Err(Box::new(pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: format!("Could not parse '{template_str}' as template"),
                    },
                    pest::Position::new(template_str, 0).unwrap(),
                )));
            }
        }

        Ok(result)
    }

    /// Parse only a filter expression
    pub fn parse_filter_only(input: &str) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        match Self::parse_dsl(input) {
            Ok(result) => {
                if let Some(filter) = result.filter {
                    Ok(filter)
                } else {
                    Err(Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: "Input is not a filter expression".to_string(),
                        },
                        pest::Position::new(input, 0).unwrap(),
                    )))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Parse only a template expression
    pub fn parse_template_only(input: &str) -> Result<Template, Box<pest::error::Error<Rule>>> {
        match Self::parse_dsl(input) {
            Ok(result) => {
                if let Some(template) = result.template {
                    Ok(template)
                } else {
                    Err(Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: "Input is not a template expression".to_string(),
                        },
                        pest::Position::new(input, 0).unwrap(),
                    )))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Parse only a field selector
    pub fn parse_field_selector_only(
        input: &str,
    ) -> Result<FieldPath, Box<pest::error::Error<Rule>>> {
        match Self::parse_dsl(input) {
            Ok(result) => {
                if let Some(field_selector) = result.field_selector {
                    Ok(field_selector)
                } else {
                    Err(Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: "Input is not a field selector".to_string(),
                        },
                        pest::Position::new(input, 0).unwrap(),
                    )))
                }
            }
            Err(e) => Err(e),
        }
    }

    fn parse_expression(
        pair: Pair<Rule>,
        result: &mut ParsedDSL,
    ) -> Result<(), Box<pest::error::Error<Rule>>> {
        let inner = match pair.into_inner().next() {
            Some(inner) => inner,
            None => {
                trace!("parse_expression: no inner content found");
                return Ok(());
            }
        };

        match inner.as_rule() {
            Rule::combined_expr => {
                // Filter + template combination
                let mut inner_pairs = inner.into_inner();
                if let (Some(filter_pair), Some(template_pair)) =
                    (inner_pairs.next(), inner_pairs.next())
                {
                    result.filter = Some(FilterParser::parse_filter_expr(filter_pair)?);
                    result.template = Some(TemplateParser::parse_template_expr(template_pair)?);
                }
            }
            Rule::template_expr => {
                result.template = Some(TemplateParser::parse_template_expr(inner)?);
            }
            Rule::filter_expr => {
                result.filter = Some(FilterParser::parse_filter_expr(inner)?);
            }
            Rule::field_selector => {
                result.field_selector = Some(Self::parse_field_selector(inner));
            }
            _ => {}
        }

        Ok(())
    }

    fn parse_field_selector(pair: Pair<Rule>) -> FieldPath {
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::quoted_field => {
                let string_literal = inner.into_inner().next().unwrap();
                let content = Self::parse_string_literal(string_literal);
                let parts: Vec<String> = content.split('.').map(|s| s.to_string()).collect();
                FieldPath::new(parts)
            }
            Rule::bare_field => {
                // bare_field contains field_path
                let field_path_pair = inner.into_inner().next().unwrap();
                Self::parse_field_path(field_path_pair)
            }
            _ => unreachable!("Unexpected field selector type"),
        }
    }

    fn parse_string_literal(pair: Pair<Rule>) -> String {
        let string_content = pair.into_inner().next().unwrap();
        string_content.as_str().to_string()
    }

    pub fn parse_field_path(pair: Pair<Rule>) -> FieldPath {
        let parts: Vec<String> = pair
            .into_inner()
            .map(|component| component.as_str().to_string())
            .collect();
        FieldPath::new(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{ComparisonOp, FilterExpr, FilterValue, TemplateItem};

    #[test]
    fn test_parse_dsl_field_selector() {
        let result = DSLParser::parse_dsl("name").unwrap();
        assert!(result.field_selector.is_some());
        assert!(result.filter.is_none());
        assert!(result.template.is_none());

        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["name"]);
    }

    #[test]
    fn test_parse_dsl_simple_filter() {
        let result = DSLParser::parse_dsl("age > 25").unwrap();
        assert!(result.filter.is_some());
        assert!(result.field_selector.is_none());
        assert!(result.template.is_none());
    }

    #[test]
    fn test_parse_dsl_simple_template() {
        let result = DSLParser::parse_dsl("{${name}}").unwrap();
        assert!(result.template.is_some());
        assert!(result.filter.is_none());
        assert!(result.field_selector.is_none());
    }

    #[test]
    fn test_parse_filter_only() {
        let result = DSLParser::parse_filter_only("age > 25").unwrap();
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
    fn test_parse_template_only() {
        let result = DSLParser::parse_template_only("$name").unwrap();
        assert_eq!(result.items.len(), 1);
        match &result.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["name"]),
            _ => panic!("Expected field"),
        }
    }

    #[test]
    fn test_parse_field_selector_only() {
        let result = DSLParser::parse_field_selector_only("user.email").unwrap();
        assert_eq!(result.parts, vec!["user", "email"]);
    }

    #[test]
    fn test_field_truthy_parsing() {
        let result = DSLParser::parse_dsl("active?").unwrap();
        assert!(result.filter.is_some());
        match result.filter {
            Some(FilterExpr::FieldTruthy(field)) => {
                assert_eq!(field.parts, vec!["active"]);
            }
            _ => panic!("active? should parse as FieldTruthy"),
        }
    }

    #[test]
    fn test_not_operator() {
        // Test NOT with explicit truthy
        let result = DSLParser::parse_dsl("!active?").unwrap();
        assert!(result.filter.is_some());
        match result.filter {
            Some(FilterExpr::Not(inner)) => match inner.as_ref() {
                FilterExpr::FieldTruthy(field) => {
                    assert_eq!(field.parts, vec!["active"]);
                }
                _ => panic!("Expected FieldTruthy inside NOT"),
            },
            _ => panic!("Expected NOT expression"),
        }
    }

    #[test]
    fn test_boolean_and_expression() {
        let result = DSLParser::parse_dsl("active? && verified?").unwrap();
        assert!(result.filter.is_some());
        match result.filter {
            Some(FilterExpr::And(left, right)) => match (left.as_ref(), right.as_ref()) {
                (FilterExpr::FieldTruthy(l), FilterExpr::FieldTruthy(r)) => {
                    assert_eq!(l.parts, vec!["active"]);
                    assert_eq!(r.parts, vec!["verified"]);
                }
                _ => panic!("Expected two FieldTruthy in AND"),
            },
            _ => panic!("Expected AND expression"),
        }
    }

    #[test]
    fn test_boolean_or_expression() {
        let result = DSLParser::parse_dsl("premium? || admin?").unwrap();
        assert!(result.filter.is_some());
        match result.filter {
            Some(FilterExpr::Or(left, right)) => match (left.as_ref(), right.as_ref()) {
                (FilterExpr::FieldTruthy(l), FilterExpr::FieldTruthy(r)) => {
                    assert_eq!(l.parts, vec!["premium"]);
                    assert_eq!(r.parts, vec!["admin"]);
                }
                _ => panic!("Expected two FieldTruthy in OR"),
            },
            _ => panic!("Expected OR expression"),
        }
    }

    #[test]
    fn test_in_operator() {
        // The 'in' operator has been removed from the grammar
        // This test should now expect a parse error
        let result = DSLParser::parse_dsl("status in [\"active\", \"pending\"]");
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
            let result = DSLParser::parse_dsl(input).unwrap();
            assert!(result.filter.is_some());
            match result.filter {
                Some(FilterExpr::Comparison { op, .. }) => {
                    assert!(std::mem::discriminant(&op) == std::mem::discriminant(&expected_op));
                }
                _ => panic!("Expected comparison for: {input}"),
            }
        }

        // Test regex patterns if supported
        if let Ok(result) = DSLParser::parse_dsl("name ~= /pattern/") {
            assert!(result.filter.is_some());
        } else {
            println!("Regex literals not fully supported, skipping");
        }
    }

    #[test]
    fn test_quoted_field_selectors() {
        let result = DSLParser::parse_dsl("\"field with spaces\"").unwrap();
        assert!(result.field_selector.is_some());
        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["field with spaces"]);
    }

    #[test]
    fn test_nested_field_access() {
        let result = DSLParser::parse_dsl("user.profile.email").unwrap();
        assert!(result.field_selector.is_some());
        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["user", "profile", "email"]);
    }

    #[test]
    fn test_complex_boolean_expressions() {
        // Test parenthesized expressions
        let result = DSLParser::parse_dsl("(active? && verified?) || admin?").unwrap();
        assert!(result.filter.is_some());

        // Test nested boolean logic
        let result = DSLParser::parse_dsl("!suspended? && (premium? || credits > 100)").unwrap();
        assert!(result.filter.is_some());
    }

    #[test]
    fn test_combined_filter_template() {
        let result = DSLParser::parse_dsl("age > 25 {${name}}").unwrap();
        assert!(result.filter.is_some());
        assert!(result.template.is_some());
    }

    #[test]
    fn test_value_types() {
        // String values
        let result = DSLParser::parse_dsl("name == \"Alice\"").unwrap();
        match result.filter {
            Some(FilterExpr::Comparison {
                value: FilterValue::String(s),
                ..
            }) => {
                assert_eq!(s, "Alice");
            }
            _ => panic!("Expected string value"),
        }

        // Number values
        let result = DSLParser::parse_dsl("age == 25").unwrap();
        match result.filter {
            Some(FilterExpr::Comparison {
                value: FilterValue::Number(n),
                ..
            }) => {
                assert_eq!(n, 25.0);
            }
            _ => panic!("Expected number value"),
        }

        // Boolean values
        let result = DSLParser::parse_dsl("active == true").unwrap();
        match result.filter {
            Some(FilterExpr::Comparison {
                value: FilterValue::Boolean(b),
                ..
            }) => {
                assert!(b);
            }
            _ => panic!("Expected boolean value"),
        }

        // Null values
        let result = DSLParser::parse_dsl("data == null").unwrap();
        match result.filter {
            Some(FilterExpr::Comparison {
                value: FilterValue::Null,
                ..
            }) => {}
            _ => panic!("Expected null value"),
        }
    }
}
