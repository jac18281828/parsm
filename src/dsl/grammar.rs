//! Main grammar parser using Pest

use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use tracing::trace;

use super::ast::ParsedDSL;
use super::filter_parser::FilterParser;
use super::template_parser::TemplateParser;
use crate::filter::FieldPath;

/// Main DSL parser using Pest grammar with conservative syntax.
///
/// This parser handles the complete parsm DSL grammar with clear disambiguation:
/// - Filter expressions require explicit comparison operators and boolean logic
/// - Template strings use `${variable}` syntax for field substitution
/// - Field selectors use bare identifiers without operators
/// - Combined filter + template expressions are parsed as separate components
///
/// Pest's grammar is the sole parser: it either matches the input or the parse
/// error propagates, ensuring predictable behavior and preventing ambiguous
/// interpretations of user input.
#[derive(Parser)]
#[grammar = "pest/parsm.pest"]
pub struct DSLParser;

impl DSLParser {
    /// Main parsing entry point - much more permissive
    pub fn parse_dsl(input: &str) -> Result<ParsedDSL, Box<pest::error::Error<Rule>>> {
        // program = { SOI ~ expression? ~ EOI }: a successful parse always
        // produces exactly one program pair.
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
        // field_selector = { quoted_field | bare_field }: always exactly
        // one inner pair.
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::quoted_field => {
                // A quoted selector is one literal key, dots included - it
                // exists precisely so a key containing "." is reachable.
                // Bare `user.name` is the nested-path form. quoted_field =
                // { string_literal }: always exactly one inner pair.
                let string_literal = inner.into_inner().next().unwrap();
                let content = Self::parse_string_literal(string_literal);
                FieldPath::single(content)
            }
            Rule::bare_field => {
                // bare_field = { field_path }: always exactly one inner pair.
                let field_path_pair = inner.into_inner().next().unwrap();
                Self::parse_field_path(field_path_pair)
            }
            _ => unreachable!("Unexpected field selector type"),
        }
    }

    fn parse_string_literal(pair: Pair<Rule>) -> String {
        // string_literal = { "\"" ~ string_content ~ "\"" | "'" ~
        // string_content_single ~ "'" }: always exactly one inner pair.
        let string_content = pair.into_inner().next().unwrap();
        Self::unescape_string_content(string_content.as_str())
    }

    /// Unescape a quoted string literal's raw content (`\"` -> `"`, `\'` ->
    /// `'`, `\\` -> `\`). Any other backslash sequence is passed through
    /// unchanged rather than silently dropping the backslash. Shared by
    /// field selectors and filter string values.
    pub(super) fn unescape_string_content(raw: &str) -> String {
        let mut result = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('"') => result.push('"'),
                    Some('\'') => result.push('\''),
                    Some('\\') => result.push('\\'),
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(c);
            }
        }
        result
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
        let result = DSLParser::parse_dsl("age > 25").unwrap();
        match result.filter.unwrap() {
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
        let result = DSLParser::parse_dsl("$name").unwrap();
        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 1);
        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["name"]),
            _ => panic!("Expected field"),
        }
    }

    #[test]
    fn test_parse_field_selector_only() {
        let result = DSLParser::parse_dsl("user.email").unwrap();
        assert_eq!(result.field_selector.unwrap().parts, vec!["user", "email"]);
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
        // The grammar has no 'in' operator.
        let result = DSLParser::parse_dsl("status in [\"active\", \"pending\"]");
        assert!(result.is_err(), "IN operator should not be supported");
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

        let result = DSLParser::parse_dsl("name ~= /pattern/").unwrap();
        assert!(result.filter.is_some());
    }

    #[test]
    fn test_quoted_field_selectors() {
        let result = DSLParser::parse_dsl("\"field with spaces\"").unwrap();
        assert!(result.field_selector.is_some());
        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["field with spaces"]);
    }

    /// A quoted selector is one literal key, dots included; only quoting
    /// reaches a key that itself contains a ".".
    #[test]
    fn quoted_selector_with_dots_is_one_key() {
        let result = DSLParser::parse_dsl("\"user.name\"").unwrap();
        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["user.name"]);
    }

    #[test]
    fn quoted_selector_unescapes_content() {
        let result = DSLParser::parse_dsl(r#""say \"hi\"""#).unwrap();
        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec![r#"say "hi""#]);
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
