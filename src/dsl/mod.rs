//! DSL Parser - Converts Pest parse tree to AST with Unambiguous Syntax
//!
//! This module provides a domain-specific language parser for parsm with clear, unambiguous
//! syntax rules. The parser converts user input into structured filter expressions, templates,
//! and field selectors with conservative, predictable behavior.
//!
//! ## Design Principles
//!
//! - **Unambiguous Syntax**: Each input pattern has exactly one interpretation
//! - **Conservative Parsing**: Only parse expressions with explicit, clear syntax
//! - **Predictable Behavior**: `name` is always a field selector, never a filter
//! - **Explicit Operations**: Filters require explicit comparison operators

mod ast;
mod filter_parser;
mod grammar;
mod operators;
mod template_parser;

pub use ast::ParsedDSL;
pub use grammar::{DSLParser, Rule};

use crate::filter::{FilterExpr, Template};

/// Main command parsing function: one argument, any DSL kind (a filter, a
/// template, a field selector, or a filter+template combination). Returns a
/// filter-only `ParsedDSL` with no template when the input is a filter
/// alone - the record pipeline prints the record's `$0` when a `ParsedDSL`
/// carries no template and no field selector.
pub fn parse_command(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    let trimmed = input.trim();
    DSLParser::parse_dsl(trimmed).map_err(|parse_error| match parse_error_hint(trimmed) {
        Some(hint) => hint.into(),
        None => parse_error.into(),
    })
}

/// A raw pest parse failure sometimes has a specific, nameable fix; try
/// each recognized shape before falling back to the parser's own message.
fn parse_error_hint(input: &str) -> Option<String> {
    bare_not_field_hint(input).or_else(|| unbalanced_bracket_hint(input))
}

/// A bare `!field` (negation without the explicit truthy `?`) fails the
/// grammar with a generic "expected comparison_op" error that doesn't name
/// the fix - negation is `!field?`. Recognize that exact shape and say so,
/// but only when `field` is itself a valid field path: `!a..b` should not
/// be told to become the equally-invalid `!a..b?`.
fn bare_not_field_hint(input: &str) -> Option<String> {
    let field = input.strip_prefix('!')?.trim();
    is_valid_field_path(field).then(|| {
        format!(
            "bare '!{field}' is not supported - negation requires the explicit truthy check '!{field}?'"
        )
    })
}

/// Whether `field` matches the grammar's `field_path` rule: one or more
/// "."-separated components, each either all ASCII digits or starting with
/// a letter/underscore.
fn is_valid_field_path(field: &str) -> bool {
    !field.is_empty()
        && field.split('.').all(|part| {
            !part.is_empty()
                && (part.chars().all(|c| c.is_ascii_digit())
                    || part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                        && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        })
}

/// A "[...]" template accepts a balanced "[...]" pair as literal (its
/// variables still interpolating) or "\[" / "\]" as an escaped literal
/// bracket; an unescaped, unbalanced bracket fails the grammar with a
/// generic "expected ..." error. Recognize that shape - skipping quoted
/// and regex-literal spans, where a bracket character has nothing to do
/// with a template - and name the escape that fixes it.
fn unbalanced_bracket_hint(input: &str) -> Option<String> {
    enum Span {
        None,
        Double,
        Single,
        Regex,
    }
    let mut span = Span::None;
    let mut depth: i32 = 0;
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        match span {
            Span::None => match c {
                '"' => span = Span::Double,
                '\'' => span = Span::Single,
                '/' => span = Span::Regex,
                '\\' => {
                    chars.next();
                }
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth < 0 {
                        return Some(
                            "unbalanced ']' in a bracketed template - use '\\]' for a literal ']'"
                                .to_string(),
                        );
                    }
                }
                _ => {}
            },
            Span::Double => {
                if c == '\\' {
                    chars.next();
                } else if c == '"' {
                    span = Span::None;
                }
            }
            Span::Single => {
                if c == '\\' {
                    chars.next();
                } else if c == '\'' {
                    span = Span::None;
                }
            }
            Span::Regex => {
                if c == '/' {
                    span = Span::None;
                }
            }
        }
    }
    (depth > 0)
        .then(|| "unbalanced '[' in a bracketed template - use '\\[' for a literal '['".to_string())
}

/// Parse the CLI's two-argument form: the first argument is a filter
/// expression or empty, the second a template. Either argument parsing as
/// any other DSL kind - a field selector, a template in the filter
/// position, a filter in the template position, or a filter+template
/// combination - is an error naming the argument and what it parsed as.
pub fn parse_separate_expressions(
    filter: Option<&str>,
    template: Option<&str>,
) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    let mut result = ParsedDSL::new();

    if let Some(filter_str) = filter
        && !filter_str.trim().is_empty()
    {
        result.filter = Some(parse_argument_as_filter(1, filter_str)?);
    }

    if let Some(template_str) = template
        && !template_str.trim().is_empty()
    {
        result.template = Some(parse_argument_as_template(2, template_str)?);
    }

    Ok(result)
}

/// Parse CLI argument `arg_num` (`raw`) and require it to be a filter
/// expression alone.
fn parse_argument_as_filter(
    arg_num: u8,
    raw: &str,
) -> Result<FilterExpr, Box<dyn std::error::Error>> {
    let trimmed = raw.trim();
    let parsed = DSLParser::parse_dsl(trimmed).map_err(|e| -> Box<dyn std::error::Error> {
        match parse_error_hint(trimmed) {
            Some(hint) => format!("argument {arg_num} ('{raw}'): {hint}").into(),
            None => format!("argument {arg_num} ('{raw}'): {e}").into(),
        }
    })?;
    match parsed {
        ParsedDSL {
            filter: Some(filter),
            template: None,
            field_selector: None,
        } => Ok(filter),
        other => Err(format!(
            "argument {arg_num} ('{raw}') is not a filter expression - it parsed as {}",
            describe_parsed(&other)
        )
        .into()),
    }
}

/// Parse CLI argument `arg_num` (`raw`) and require it to be a template
/// alone.
fn parse_argument_as_template(
    arg_num: u8,
    raw: &str,
) -> Result<Template, Box<dyn std::error::Error>> {
    let trimmed = raw.trim();
    let parsed = DSLParser::parse_dsl(trimmed).map_err(|e| -> Box<dyn std::error::Error> {
        match parse_error_hint(trimmed) {
            Some(hint) => format!("argument {arg_num} ('{raw}'): {hint}").into(),
            None => format!("argument {arg_num} ('{raw}'): {e}").into(),
        }
    })?;
    match parsed {
        ParsedDSL {
            filter: None,
            template: Some(template),
            field_selector: None,
        } => Ok(template),
        other => Err(format!(
            "argument {arg_num} ('{raw}') is not a template - it parsed as {}",
            describe_parsed(&other)
        )
        .into()),
    }
}

/// Name the DSL kind a `ParsedDSL` resolved to, for an argument-position
/// mismatch error. `expression`'s grammar only ever sets more than one of
/// `filter`/`template`/`field_selector` via `combined_expr`, so a filter and
/// a field selector are never both set.
fn describe_parsed(dsl: &ParsedDSL) -> &'static str {
    match (
        dsl.filter.is_some(),
        dsl.template.is_some(),
        dsl.field_selector.is_some(),
    ) {
        (true, true, _) => "a combined filter and template",
        (true, false, false) => "a filter expression",
        (false, true, false) => "a template",
        (false, false, true) => "a field selector",
        (false, false, false) => "nothing",
        (true, false, true) | (false, true, true) => "an unexpected combination",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{FilterEngine, FilterExpr, TemplateItem};
    use serde_json::json;

    #[test]
    fn test_parse_command_field_selector() {
        let result = parse_command("name").unwrap();
        assert!(result.field_selector.is_some());
        assert!(result.filter.is_none());
        assert!(result.template.is_none());

        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["name"]);
    }

    #[test]
    fn test_parse_command_simple_filter() {
        let result = parse_command("age > 25").unwrap();
        assert!(result.filter.is_some());
        assert!(result.field_selector.is_none());
        // No default template injection: a filter-only expression carries no
        // template; the record pipeline prints the record's `$0` for that case.
        assert!(result.template.is_none());
    }

    #[test]
    fn test_parse_command_simple_template() {
        let result = parse_command("{${name}}").unwrap();
        assert!(result.template.is_some());
        assert!(result.filter.is_none());
        assert!(result.field_selector.is_none());
    }

    #[test]
    fn test_parse_command_combined_filter_template() {
        let result = parse_command("age > 25 {${name}}").unwrap();
        assert!(result.filter.is_some());
        assert!(result.template.is_some());
        assert!(result.field_selector.is_none());
    }

    #[test]
    fn test_new_template_syntax() {
        // Test braced templates with explicit field substitution
        let result = parse_command("{${name}}").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 1);
        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["name"]),
            _ => panic!("Expected field substitution"),
        }

        // Test bracketed templates
        let result = parse_command("[${name}]").unwrap();
        assert!(result.template.is_some());

        // Test simple variables
        let result = parse_command("$name").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 1);
        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["name"]),
            _ => panic!("Expected field substitution"),
        }

        // Test interpolated text in brackets
        let result = parse_command("[Hello ${name}!]").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        // Adjust expectations - the parser might segment this differently
        assert!(template.items.len() >= 2); // At least literal + field
    }

    #[test]
    fn test_field_truthy_parsing() {
        // The grammar already has field_truthy rule, test it works
        let result = parse_command("active?").unwrap();
        assert!(result.filter.is_some());
        match result.filter {
            Some(FilterExpr::FieldTruthy(field)) => {
                assert_eq!(field.parts, vec!["active"]);
            }
            _ => panic!("active? should parse as FieldTruthy"),
        }

        // Test nested field truthy
        let result = parse_command("user.settings.notifications?").unwrap();
        assert!(result.filter.is_some());
        match result.filter {
            Some(FilterExpr::FieldTruthy(field)) => {
                assert_eq!(field.parts, vec!["user", "settings", "notifications"]);
            }
            _ => panic!("user.settings.notifications? should parse as FieldTruthy"),
        }
    }

    #[test]
    fn test_explicit_truthy_in_boolean_expressions() {
        // AND with explicit truthy
        let result = parse_command("active? && verified?").unwrap();
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

        // OR with explicit truthy
        let result = parse_command("premium? || admin?").unwrap();
        assert!(result.filter.is_some());

        // NOT with truthy
        let result = parse_command("!suspended?").unwrap();
        assert!(result.filter.is_some());
        match result.filter {
            Some(FilterExpr::Not(inner)) => match inner.as_ref() {
                FilterExpr::FieldTruthy(field) => {
                    assert_eq!(field.parts, vec!["suspended"]);
                }
                _ => panic!("Expected FieldTruthy inside NOT"),
            },
            _ => panic!("Expected NOT expression"),
        }
    }

    #[test]
    fn test_not_operator_without_truthy() {
        // Bare `!field` (no `?`) is rejected everywhere: negation requires
        // the explicit truthy check.
        assert!(parse_command("!active").is_err());
        assert!(parse_command("!!verified").is_err());

        // NOT with explicit truthy syntax works, standalone and chained.
        let result = parse_command("!active?").unwrap();
        assert!(result.filter.is_some());
        let result = parse_command("!active? && !suspended?").unwrap();
        assert!(result.filter.is_some());
    }

    #[test]
    fn test_mixed_syntax() {
        // Mix truthy with comparisons
        let result = parse_command("active? && age > 18").unwrap();
        assert!(result.filter.is_some());

        // Mix comparisons with truthy
        let result = parse_command("name == \"Alice\" || admin?").unwrap();
        assert!(result.filter.is_some());

        // Complex mixed
        let result = parse_command("(premium? || credits > 100) && !blacklisted?").unwrap();
        assert!(result.filter.is_some());
    }

    #[test]
    fn test_in_operator() {
        // The grammar has no 'in' operator.
        let result = parse_command("status in [\"active\", \"pending\"]");
        assert!(result.is_err(), "IN operator should not be supported");
    }

    #[test]
    fn test_real_world_scenarios() {
        // Test simpler versions of real-world scenarios

        // Simple user permissions check
        let result = parse_command("authenticated? && !banned?").unwrap();
        assert!(result.filter.is_some());

        // Simple content filtering
        let result = parse_command("published? && rating >= 4.0").unwrap();
        assert!(result.filter.is_some());

        // Simple business logic
        let result = parse_command("age >= 18 && premium_member?").unwrap();
        assert!(result.filter.is_some());
    }

    #[test]
    fn test_existing_template_preserved() {
        // Field selectors
        assert!(parse_command("username").unwrap().field_selector.is_some());
        assert!(
            parse_command("user.profile.bio")
                .unwrap()
                .field_selector
                .is_some()
        );
        assert!(
            parse_command("\"field with spaces\"")
                .unwrap()
                .field_selector
                .is_some()
        );

        // Templates
        assert!(parse_command("$name").unwrap().template.is_some());
        assert!(parse_command("{${user.name}}").unwrap().template.is_some());
        assert!(
            parse_command("[Hello ${name}!]")
                .unwrap()
                .template
                .is_some()
        );

        // Comparisons
        assert!(parse_command("age >= 21").unwrap().filter.is_some());
        assert!(
            parse_command("status != \"deleted\"")
                .unwrap()
                .filter
                .is_some()
        );
        assert!(parse_command("score > 0.5").unwrap().filter.is_some());

        // Combined
        let result = parse_command("score > 90 {Congrats ${name}!}").unwrap();
        assert!(result.filter.is_some() && result.template.is_some());
    }

    #[test]
    fn test_conservative_boolean_parsing() {
        // Test explicit truthy syntax works for any field
        let test_fields = [
            "a", "b", "field_1", "field_2", "name", "active", "x", "y", "z",
        ];

        // These should ALL work with ? syntax
        for &field1 in &test_fields {
            for &field2 in &test_fields {
                // Explicit truthy - should work
                let command = format!("{field1}? && {field2}?");
                match parse_command(&command) {
                    Ok(result) => {
                        assert!(result.filter.is_some(), "{command} should parse as filter");
                    }
                    Err(e) => panic!("{command} should work with ? syntax: {e}"),
                }

                // Bare fields - should NOT work as filter
                let command = format!("{field1} && {field2}");
                match parse_command(&command) {
                    Ok(result) => {
                        assert!(
                            result.filter.is_none(),
                            "{command} should NOT parse as filter - ambiguous"
                        );
                    }
                    Err(_) => {
                        // Expected - grammar rejects this
                    }
                }
            }
        }

        // NOT without ? should work (explicit operator)
        for &field in &test_fields {
            let command = format!("!{field}?"); // Use explicit truthy syntax
            match parse_command(&command) {
                Ok(result) => {
                    assert!(result.filter.is_some(), "{command} should parse as filter");
                }
                Err(e) => panic!("{command} should work - NOT with ? is explicit: {e}"),
            }
        }
    }

    #[test]
    fn test_template_variable_edge_cases() {
        // ${0} is the special variable for the record's original input.
        let template = parse_command("${0}").unwrap().template.unwrap();
        assert_eq!(
            template.render(&json!({"$0": "original text"})),
            "original text"
        );

        // $0 (unbraced) is a literal dollar amount, not the special variable.
        let template = parse_command("$0").unwrap().template.unwrap();
        assert_eq!(template.render(&json!({"$0": "original text"})), "$0");

        // $20 is a literal dollar amount.
        let template = parse_command("$20").unwrap().template.unwrap();
        assert_eq!(template.render(&json!({})), "$20");

        // ${1} is a field variable, mapped to the field named "1".
        let template = parse_command("${1}").unwrap().template.unwrap();
        assert_eq!(template.render(&json!({"1": "first field"})), "first field");
    }

    #[test]
    fn test_mixed_numeric_template_patterns() {
        // A "$digits" amount inside a braced template stays literal text;
        // it does not split at the dollar sign or consume the variable.
        let template = parse_command("{I have $20 and ${name} has $100}")
            .unwrap()
            .template
            .unwrap();
        assert_eq!(
            template.render(&json!({"name": "Bob"})),
            "I have $20 and Bob has $100"
        );

        // Same for a bracketed template.
        let template = parse_command("[Hello ${name}, you owe $25]")
            .unwrap()
            .template
            .unwrap();
        assert_eq!(
            template.render(&json!({"name": "Eve"})),
            "Hello Eve, you owe $25"
        );
    }

    #[test]
    fn test_quoted_string_literals() {
        // "Alice" is a field selector for the literal key "Alice", not a
        // filter or template.
        let result = parse_command("\"Alice\"").unwrap();
        assert!(result.filter.is_none());
        assert!(result.template.is_none());
        let field_selector = result.field_selector.unwrap();
        assert_eq!(
            field_selector.get_value(&json!({"Alice": "found"})),
            Some(&json!("found"))
        );

        // Single-quoted strings select the same way.
        let field_selector = parse_command("'Alice'").unwrap().field_selector.unwrap();
        assert_eq!(
            field_selector.get_value(&json!({"Alice": "found"})),
            Some(&json!("found"))
        );
    }

    #[test]
    fn test_error_cases() {
        // Test that bare fields in boolean context are rejected
        let test_cases = vec!["active && verified", "field || other", "(name && active)"];

        for input in test_cases {
            match parse_command(input) {
                Ok(result) => {
                    if result.filter.is_some() {
                        panic!("{input} should not parse as filter - ambiguous");
                    }
                    // OK if it doesn't parse as filter
                }
                Err(e) => {
                    let msg = e.to_string();
                    // Should suggest using ? or comparison
                    assert!(
                        msg.contains("?")
                            || msg.contains("truthy")
                            || msg.contains("comparison")
                            || msg.contains("explicit")
                            || msg.contains("field name")
                            || msg.contains("Template"),
                        "Error for '{input}' should be helpful: {msg}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_complex_filters() {
        let result = parse_command("name == \"Alice\" && age > 25").unwrap();
        assert!(matches!(result.filter, Some(FilterExpr::And(_, _))));
    }

    #[test]
    fn test_nested_field_access() {
        // In filters: a nested field resolves through the comparison.
        let filter = parse_command("user.email == \"alice@example.com\"")
            .unwrap()
            .filter
            .unwrap();
        assert!(FilterEngine::evaluate(
            &filter,
            &json!({"user": {"email": "alice@example.com"}})
        ));
        assert!(!FilterEngine::evaluate(
            &filter,
            &json!({"user": {"email": "bob@example.com"}})
        ));

        // In templates: a nested field renders its resolved value.
        let template = parse_command("{${user.name}}").unwrap().template.unwrap();
        assert_eq!(
            template.render(&json!({"user": {"name": "Alice"}})),
            "Alice"
        );

        // In field selectors: a nested path navigates to the leaf value.
        let field = parse_command("user.profile.bio")
            .unwrap()
            .field_selector
            .unwrap();
        assert_eq!(
            field.get_value(&json!({"user": {"profile": {"bio": "hi"}}})),
            Some(&json!("hi"))
        );
    }

    #[test]
    fn test_special_field_references() {
        // Test $0 (original input) field reference
        let result = parse_command("${0}").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["$0"]),
            _ => panic!("Expected $0 field reference"),
        }

        // Test numeric field references
        let result = parse_command("${1}").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["1"]),
            _ => panic!("Expected numeric field reference"),
        }
    }

    #[test]
    fn test_comprehensive_disambiguation() {
        // Identical field names are interpreted as different DSL kinds
        // depending on context; each kind is exercised, not just detected.
        let data = json!({"name": "Alice"});

        // "name" as field selector.
        let result = parse_command("name").unwrap();
        assert!(result.filter.is_none());
        assert!(result.template.is_none());
        assert_eq!(
            result.field_selector.unwrap().get_value(&data),
            Some(&json!("Alice"))
        );

        // "name?" as filter (truthy check) - no template, same as any
        // other filter-only expression.
        let result = parse_command("name?").unwrap();
        assert!(result.field_selector.is_none());
        assert!(result.template.is_none());
        assert!(FilterEngine::evaluate(&result.filter.unwrap(), &data));
        assert!(!FilterEngine::evaluate(
            &parse_command("name?").unwrap().filter.unwrap(),
            &json!({"name": ""})
        ));

        // "$name" as template.
        let result = parse_command("$name").unwrap();
        assert!(result.filter.is_none());
        assert!(result.field_selector.is_none());
        assert_eq!(result.template.unwrap().render(&data), "Alice");

        // "{${name}}" as template.
        let result = parse_command("{${name}}").unwrap();
        assert!(result.filter.is_none());
        assert!(result.field_selector.is_none());
        assert_eq!(result.template.unwrap().render(&data), "Alice");

        // "name == \"Alice\"" as filter - no template.
        let result = parse_command("name == \"Alice\"").unwrap();
        assert!(result.field_selector.is_none());
        assert!(result.template.is_none());
        assert!(FilterEngine::evaluate(&result.filter.unwrap(), &data));
        assert!(!FilterEngine::evaluate(
            &parse_command("name == \"Alice\"").unwrap().filter.unwrap(),
            &json!({"name": "Bob"})
        ));
    }

    #[test]
    fn test_edge_cases() {
        // Empty braces should still be valid templates
        let result = parse_command("{}").unwrap();
        assert!(result.template.is_some());

        // Empty brackets should still be valid templates
        let result = parse_command("[]").unwrap();
        assert!(result.template.is_some());

        // Single characters should be field selectors
        let result = parse_command("a").unwrap();
        assert!(result.field_selector.is_some());

        // Numbers should be field selectors
        let result = parse_command("1").unwrap();
        assert!(result.field_selector.is_some());
    }

    #[test]
    fn test_bracketed_template_syntax() {
        // Bracketed templates render like braced templates.
        let template = parse_command("[${name}]").unwrap().template.unwrap();
        assert_eq!(template.render(&json!({"name": "Alice"})), "Alice");

        // Mixed literal and field content in brackets.
        let template = parse_command("[Hello ${name}!]").unwrap().template.unwrap();
        assert_eq!(template.render(&json!({"name": "Alice"})), "Hello Alice!");

        // Combined with a filter: both the filter and the template render.
        let result = parse_command("age > 25 [User: ${name}]").unwrap();
        let filter = result.filter.unwrap();
        let template = result.template.unwrap();
        let data = json!({"age": 30, "name": "Alice"});
        assert!(FilterEngine::evaluate(&filter, &data));
        assert_eq!(template.render(&data), "User: Alice");
        assert!(!FilterEngine::evaluate(
            &filter,
            &json!({"age": 10, "name": "Alice"})
        ));
    }

    #[test]
    fn test_numeric_literal_vs_field_distinction() {
        // Test that dollar amounts are preserved as literals
        for amount in ["$0", "$1", "$5", "$10", "$20", "$100", "$999"] {
            let result = parse_command(amount).unwrap();
            assert!(result.template.is_some());
            let template = result.template.unwrap();
            match &template.items[0] {
                TemplateItem::Literal(text) => assert_eq!(text, amount),
                _ => panic!("Expected {amount} to be literal"),
            }
        }

        // Test that braced numerics are field references
        for num in ["${1}", "${2}", "${10}", "${100}"] {
            let result = parse_command(num).unwrap();
            assert!(result.template.is_some());
            let template = result.template.unwrap();
            match &template.items[0] {
                TemplateItem::Field(_) => {} // Expected
                _ => panic!("Expected {num} to be field reference"),
            }
        }
    }
}
