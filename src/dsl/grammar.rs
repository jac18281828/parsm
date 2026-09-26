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
    use crate::filter::{ComparisonOp, FilterEngine, FilterExpr, TemplateItem};
    use serde_json::json;

    #[test]
    fn test_parse_dsl_field_selector() {
        let result = DSLParser::parse_dsl("name").unwrap();
        assert!(result.field_selector.is_some());
        assert!(result.filter.is_none());
        assert!(result.template.is_none());

        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["name"]);
        assert_eq!(
            field.get_value(&json!({"name": "Alice"})),
            Some(&json!("Alice"))
        );
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
        let filter = DSLParser::parse_dsl("age > 25").unwrap().filter.unwrap();
        match &filter {
            FilterExpr::Comparison { field, op, .. } => {
                assert_eq!(field.parts, vec!["age"]);
                assert!(matches!(op, ComparisonOp::GreaterThan));
            }
            _ => panic!("Expected comparison"),
        }
        assert!(FilterEngine::evaluate(&filter, &json!({"age": 30})));
        assert!(!FilterEngine::evaluate(&filter, &json!({"age": 10})));
    }

    #[test]
    fn test_parse_template_only() {
        let result = DSLParser::parse_dsl("$name").unwrap();
        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 1);
        assert!(matches!(&template.items[0], TemplateItem::Field(_)));
        assert_eq!(template.render(&json!({"name": "Alice"})), "Alice");
    }

    #[test]
    fn test_parse_field_selector_only() {
        let field = DSLParser::parse_dsl("user.email")
            .unwrap()
            .field_selector
            .unwrap();
        assert_eq!(field.parts, vec!["user", "email"]);
        assert_eq!(
            field.get_value(&json!({"user": {"email": "a@b.com"}})),
            Some(&json!("a@b.com"))
        );
    }

    #[test]
    fn test_field_truthy_parsing() {
        let filter = DSLParser::parse_dsl("active?").unwrap().filter.unwrap();
        assert!(FilterEngine::evaluate(&filter, &json!({"active": true})));
        assert!(!FilterEngine::evaluate(&filter, &json!({"active": false})));
    }

    #[test]
    fn test_not_operator() {
        let filter = DSLParser::parse_dsl("!active?").unwrap().filter.unwrap();
        assert!(FilterEngine::evaluate(&filter, &json!({"active": false})));
        assert!(!FilterEngine::evaluate(&filter, &json!({"active": true})));
    }

    #[test]
    fn test_boolean_and_expression() {
        let filter = DSLParser::parse_dsl("active? && verified?")
            .unwrap()
            .filter
            .unwrap();
        assert!(FilterEngine::evaluate(
            &filter,
            &json!({"active": true, "verified": true})
        ));
        assert!(!FilterEngine::evaluate(
            &filter,
            &json!({"active": true, "verified": false})
        ));
        assert!(!FilterEngine::evaluate(
            &filter,
            &json!({"active": false, "verified": true})
        ));
    }

    #[test]
    fn test_boolean_or_expression() {
        let filter = DSLParser::parse_dsl("premium? || admin?")
            .unwrap()
            .filter
            .unwrap();
        assert!(FilterEngine::evaluate(
            &filter,
            &json!({"premium": true, "admin": false})
        ));
        assert!(FilterEngine::evaluate(
            &filter,
            &json!({"premium": false, "admin": true})
        ));
        assert!(!FilterEngine::evaluate(
            &filter,
            &json!({"premium": false, "admin": false})
        ));
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
        let field = DSLParser::parse_dsl("\"field with spaces\"")
            .unwrap()
            .field_selector
            .unwrap();
        assert_eq!(
            field.get_value(&json!({"field with spaces": "found"})),
            Some(&json!("found"))
        );
    }

    /// A quoted selector is one literal key, dots included; only quoting
    /// reaches a key that itself contains a ".".
    #[test]
    fn quoted_selector_with_dots_is_one_key() {
        let field = DSLParser::parse_dsl("\"user.name\"")
            .unwrap()
            .field_selector
            .unwrap();
        assert_eq!(field.parts, vec!["user.name"]);
        assert_eq!(
            field.get_value(&json!({"user.name": "Alice"})),
            Some(&json!("Alice"))
        );
    }

    #[test]
    fn quoted_selector_unescapes_content() {
        let field = DSLParser::parse_dsl(r#""say \"hi\"""#)
            .unwrap()
            .field_selector
            .unwrap();
        assert_eq!(
            field.get_value(&json!({r#"say "hi""#: "found"})),
            Some(&json!("found"))
        );
    }

    #[test]
    fn test_nested_field_access() {
        let field = DSLParser::parse_dsl("user.profile.email")
            .unwrap()
            .field_selector
            .unwrap();
        assert_eq!(
            field.get_value(&json!({"user": {"profile": {"email": "a@b.com"}}})),
            Some(&json!("a@b.com"))
        );
    }

    #[test]
    fn test_complex_boolean_expressions() {
        // Parenthesized OR: either clause satisfies the whole expression.
        let filter = DSLParser::parse_dsl("(active? && verified?) || admin?")
            .unwrap()
            .filter
            .unwrap();
        assert!(FilterEngine::evaluate(
            &filter,
            &json!({"active": true, "verified": true, "admin": false})
        ));
        assert!(FilterEngine::evaluate(
            &filter,
            &json!({"active": false, "verified": false, "admin": true})
        ));
        assert!(!FilterEngine::evaluate(
            &filter,
            &json!({"active": false, "verified": false, "admin": false})
        ));

        // Negation gates a parenthesized OR.
        let filter = DSLParser::parse_dsl("!suspended? && (premium? || credits > 100)")
            .unwrap()
            .filter
            .unwrap();
        assert!(FilterEngine::evaluate(
            &filter,
            &json!({"suspended": false, "premium": true, "credits": 0})
        ));
        assert!(!FilterEngine::evaluate(
            &filter,
            &json!({"suspended": true, "premium": true, "credits": 1000})
        ));
        assert!(!FilterEngine::evaluate(
            &filter,
            &json!({"suspended": false, "premium": false, "credits": 50})
        ));
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
        let filter = DSLParser::parse_dsl("name == \"Alice\"")
            .unwrap()
            .filter
            .unwrap();
        assert!(FilterEngine::evaluate(&filter, &json!({"name": "Alice"})));
        assert!(!FilterEngine::evaluate(&filter, &json!({"name": "Bob"})));

        // Number values
        let filter = DSLParser::parse_dsl("age == 25").unwrap().filter.unwrap();
        assert!(FilterEngine::evaluate(&filter, &json!({"age": 25})));
        assert!(!FilterEngine::evaluate(&filter, &json!({"age": 30})));

        // Boolean values
        let filter = DSLParser::parse_dsl("active == true")
            .unwrap()
            .filter
            .unwrap();
        assert!(FilterEngine::evaluate(&filter, &json!({"active": true})));
        assert!(!FilterEngine::evaluate(&filter, &json!({"active": false})));

        // Null values
        let filter = DSLParser::parse_dsl("data == null")
            .unwrap()
            .filter
            .unwrap();
        assert!(FilterEngine::evaluate(&filter, &json!({"data": null})));
        assert!(!FilterEngine::evaluate(
            &filter,
            &json!({"data": "present"})
        ));
    }
}
