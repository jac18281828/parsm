//! Fallback parsing strategies for DSL expressions

use tracing::trace;

use super::ast::ParsedDSL;
use super::grammar::DSLParser;
use super::template_parser::TemplateParser;
use crate::filter::{ComparisonOp, FieldPath, FilterExpr, FilterValue, Template, TemplateItem};

/// Try fallback parsing strategies when the main parser fails
pub fn try_fallback_parsing(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    // Strategy 1: Try manual parsing for complex cases (filter+template combinations)
    if let Ok(result) = try_manual_parsing(input) {
        trace!("Manual parsing strategy succeeded");
        return Ok(result);
    }

    // Strategy 2: Try boolean expressions with truthy fields
    if let Ok(result) = try_boolean_with_truthy_fields(input) {
        trace!("Boolean with truthy fields strategy succeeded");
        return Ok(result);
    }

    // Strategy 3: Try as simple template patterns
    if let Ok(result) = try_simple_template_patterns(input) {
        trace!("Simple template patterns strategy succeeded");
        return Ok(result);
    }

    // Strategy 4: Try as field selector
    if let Ok(result) = try_as_field_selector(input) {
        trace!("Field selector strategy succeeded");
        return Ok(result);
    }

    trace!("All parsing strategies failed");

    // If all else fails, provide a helpful error
    Err(format!(
        "Could not parse '{input}'. Try:\n  - Templates: [{{${{name}}}}], $name, or [Hello ${{name}}]\n  - Filters: name == \"value\" or age > 25\n  - Field selectors: name or \"field name\""
    ).into())
}

/// Try to parse simple template patterns that might not fit the grammar
fn try_simple_template_patterns(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    trace!("try_simple_template_patterns called with: '{}'", input);
    let mut result = ParsedDSL::new();

    // Pattern: {${variable}} or {$variable} or {mixed content with variables}
    if input.starts_with('{') && input.ends_with('}') {
        trace!("Found braced template pattern");
        let content = &input[1..input.len() - 1];
        if !content.trim().is_empty() {
            // Only treat as template if it contains variables ($)
            if content.contains('$') {
                trace!("Braced content contains variables, parsing manually");
                // Try to parse the content manually for variables
                if let Ok(template) = TemplateParser::parse_template_content_manually(content) {
                    result.template = Some(template);
                    return Ok(result);
                }
            }
            // If no variables, treat as literal template content
            trace!("Braced content has no variables, treating as literal");
            result.template = Some(Template {
                items: vec![TemplateItem::Literal(content.to_string())],
            });
            return Ok(result);
        }
    }

    // Pattern: $anything -> treat as variable (both $name and ${name} patterns)
    if input.starts_with('$') && !input.contains(' ') {
        trace!("Found $variable pattern");

        if input.starts_with("${") && input.ends_with("}") {
            // ${name} pattern - ALL ${...} patterns are field substitutions, including ${0}, ${1}, etc.
            let field_name = &input[2..input.len() - 1]; // Remove ${ and }
            if !field_name.is_empty() {
                trace!(
                    "Parsing as braced variable (field substitution): '{}'",
                    field_name
                );
                let field_path = parse_field_name_simple(field_name);
                result.template = Some(Template {
                    items: vec![TemplateItem::Field(field_path)],
                });
                return Ok(result);
            } else {
                trace!("Empty braced variable, treating as literal");
            }
        } else {
            // $name pattern - ONLY non-numeric $name patterns are variables
            let field_name = &input[1..];
            if !field_name.is_empty() && !field_name.chars().all(|c| c.is_ascii_digit()) {
                trace!("Parsing as simple variable: '{}'", field_name);
                let field_path = parse_field_name_simple(field_name);
                result.template = Some(Template {
                    items: vec![TemplateItem::Field(field_path)],
                });
                return Ok(result);
            } else if field_name.chars().all(|c| c.is_ascii_digit()) {
                trace!(
                    "Dollar sign followed by digits only ($0, $1, $20, etc.), treating as numeric literal"
                );
                result.template = Some(Template {
                    items: vec![TemplateItem::Literal(input.to_string())],
                });
                return Ok(result);
            } else {
                trace!("Empty dollar sign, treating as literal");
                result.template = Some(Template {
                    items: vec![TemplateItem::Literal(input.to_string())],
                });
                return Ok(result);
            }
        }
    }

    // Pattern: text with $variables -> interpolated template
    // REJECT: Bare interpolated text is not allowed. Must be in brackets or braces.
    if input.contains('$') && !input.starts_with('{') && !input.starts_with('[') {
        trace!(
            "Found potential interpolated text pattern, but bare interpolated text is not allowed"
        );
        return Err(
            "Bare interpolated text not allowed. Use [Hello ${name}] or {Hello ${name}} instead"
                .into(),
        );
    }

    Err("Not a simple template pattern".into())
}

/// Try to parse as field selector
fn try_as_field_selector(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    let mut result = ParsedDSL::new();

    // Quoted field selector
    if (input.starts_with('"') && input.ends_with('"'))
        || (input.starts_with('\'') && input.ends_with('\''))
    {
        let content = &input[1..input.len() - 1];
        let parts: Vec<String> = content.split('.').map(|s| s.to_string()).collect();
        result.field_selector = Some(FieldPath::new(parts));
        return Ok(result);
    }

    // Simple identifier (should only be field selectors, not filters)
    if input
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        && !input.is_empty()
        && !input.contains(' ')
        && !input.contains('$')
        && !input.contains('{')
        && !input.contains('}')
        && !input.contains('!')
        && !input.contains('=')
        && !input.contains('>')
        && !input.contains('<')
        && !input.contains('&')
        && !input.contains('|')
    {
        let parts: Vec<String> = input.split('.').map(|s| s.to_string()).collect();
        result.field_selector = Some(FieldPath::new(parts));
        return Ok(result);
    }

    Err("Not a field selector".into())
}

/// Try manual parsing for complex cases
fn try_manual_parsing(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    // Look for filter + template combinations manually
    if let Some((filter_part, template_part)) = split_filter_template_manually(input) {
        let mut result = ParsedDSL::new();

        // Try to parse filter part - attempt boolean parsing first
        let filter_parsed = if let Ok(boolean_result) = try_boolean_with_truthy_fields(filter_part)
        {
            result.filter = boolean_result.filter;
            result.filter.is_some()
        } else if let Ok(filter) = parse_simple_filter(filter_part) {
            result.filter = Some(filter);
            true
        } else {
            // Try to parse with the main parser as a fallback for filters
            if let Ok(filter_result) = DSLParser::parse_filter_only(filter_part) {
                result.filter = Some(filter_result);
                true
            } else {
                false
            }
        };

        // Try to parse template part - first try with the main parser, then fallback to simpler methods
        let template_parsed = if let Ok(template) = DSLParser::parse_template_only(template_part) {
            result.template = Some(template);
            true
        } else if let Ok(template_result) = try_simple_template_patterns(template_part) {
            result.template = template_result.template;
            result.template.is_some()
        } else if template_part.starts_with('{') && template_part.ends_with('}')
            || template_part.starts_with('[') && template_part.ends_with(']')
        {
            // Try a very basic template parse for braced/bracketed content
            let content = &template_part[1..template_part.len() - 1];

            // Try to parse the template content manually
            if let Ok(template) = TemplateParser::parse_template_content_manually(content) {
                result.template = Some(template);
                true
            } else {
                // Treat as literal template
                result.template = Some(Template {
                    items: vec![TemplateItem::Literal(content.to_string())],
                });
                true
            }
        } else {
            false
        };

        if filter_parsed && template_parsed {
            return Ok(result);
        } else if filter_parsed {
            // At least the filter was parsed, return that
            return Ok(result);
        }
    }

    // Try parsing as simple filter directly
    if let Ok(filter) = parse_simple_filter(input) {
        let mut result = ParsedDSL::new();
        result.filter = Some(filter);
        return Ok(result);
    }

    Err("Could not manually parse".into())
}

/// Try to parse boolean expressions with truthy fields
fn try_boolean_with_truthy_fields(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    // Very conservative approach: only handle expressions with explicit boolean operators
    // AND only if the expression contains some explicit comparisons OR looks like pure field boolean logic
    trace!("try_boolean_with_truthy_fields: parsing '{}'", input);

    if input.contains("&&") {
        trace!("try_boolean_with_truthy_fields: found &&, trying AND expression");
        return try_parse_and_expression(input);
    }

    if input.contains("||") {
        trace!("try_boolean_with_truthy_fields: found ||, trying OR expression");
        return try_parse_or_expression(input);
    }

    trace!("try_boolean_with_truthy_fields: no explicit boolean operators found");
    // Don't try to handle !field or bare field names as boolean expressions
    Err("Not a boolean expression with explicit operators".into())
}

/// Parse AND expressions like "field1 && field2"
fn try_parse_and_expression(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    trace!("try_parse_and_expression: parsing '{}'", input);
    let parts: Vec<&str> = input.split("&&").map(|s| s.trim()).collect();
    if parts.len() != 2 {
        trace!(
            "try_parse_and_expression: found {} parts, only 2 supported",
            parts.len()
        );
        return Err("Complex AND expressions not supported".into());
    }

    trace!(
        "try_parse_and_expression: left='{}', right='{}'",
        parts[0], parts[1]
    );

    // Try to parse each part as either a comparison or a truthy field
    let left = try_parse_single_boolean_term(parts[0])?;
    let right = try_parse_single_boolean_term(parts[1])?;

    trace!("try_parse_and_expression: successfully parsed both terms, creating AND");
    let mut result = ParsedDSL::new();
    result.filter = Some(FilterExpr::And(Box::new(left), Box::new(right)));
    Ok(result)
}

/// Parse OR expressions like "field1 || field2"  
fn try_parse_or_expression(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split("||").map(|s| s.trim()).collect();
    if parts.len() != 2 {
        return Err("Complex OR expressions not supported".into());
    }

    let left = try_parse_single_boolean_term(parts[0])?;
    let right = try_parse_single_boolean_term(parts[1])?;

    let mut result = ParsedDSL::new();
    result.filter = Some(FilterExpr::Or(Box::new(left), Box::new(right)));
    Ok(result)
}

/// Parse a single boolean term - either a comparison or a truthy field
fn try_parse_single_boolean_term(term: &str) -> Result<FilterExpr, Box<dyn std::error::Error>> {
    let trimmed = term.trim();
    trace!("try_parse_single_boolean_term: parsing '{}'", trimmed);

    // Handle NOT operations
    if let Some(stripped) = trimmed.strip_prefix('!') {
        let field_part = stripped.trim();
        trace!(
            "try_parse_single_boolean_term: found NOT operation on '{}'",
            field_part
        );

        // For NOT, we allow any field (not just known boolean fields)
        let field_path = parse_field_name_for_truthy(field_part);
        trace!(
            "try_parse_single_boolean_term: creating NOT(FieldTruthy) for '{}'",
            field_part
        );
        return Ok(FilterExpr::Not(Box::new(FilterExpr::FieldTruthy(
            field_path,
        ))));
    }

    // If it contains comparison operators, try to parse as normal
    if super::operators::contains_filter_operators(trimmed) {
        trace!(
            "try_parse_single_boolean_term: '{}' contains filter operators, trying normal parse",
            trimmed
        );
        // Try to parse as a filter expression
        if let Ok(result) = DSLParser::parse_dsl(trimmed) {
            if let Some(filter) = result.filter {
                trace!(
                    "try_parse_single_boolean_term: successfully parsed '{}' as filter",
                    trimmed
                );
                return Ok(filter);
            }
        }
        trace!(
            "try_parse_single_boolean_term: failed to parse '{}' as filter despite operators",
            trimmed
        );
    }

    // Check if it ends with '?' for explicit truthy check
    if let Some(field_name) = trimmed.strip_suffix('?') {
        let field_path = parse_field_name_for_truthy(field_name);
        trace!(
            "try_parse_single_boolean_term: creating FieldTruthy for '{}'",
            trimmed
        );
        return Ok(FilterExpr::FieldTruthy(field_path));
    }

    // Bare field names without ? are NOT allowed in boolean context
    trace!(
        "try_parse_single_boolean_term: '{}' not recognized as valid boolean term, rejecting",
        trimmed
    );
    Err(format!(
        "Cannot parse '{trimmed}' as boolean term - use 'field?' for truthy check or add comparison operator"
    )
    .into())
}

// Helper functions

fn parse_field_name_simple(field_name: &str) -> FieldPath {
    trace!("parse_field_name_simple called with: '{}'", field_name);

    // Handle special case for "0" -> "$0" (original input)
    if field_name == "0" {
        return FieldPath::new(vec!["$0".to_string()]);
    }

    // Handle numeric field references (1, 2, 3, etc. stay as "1", "2", "3")
    if let Ok(index) = field_name.parse::<usize>() {
        if index > 0 {
            trace!("Numeric field {} stays as is", index);
            return FieldPath::new(vec![field_name.to_string()]);
        }
    }

    let parts: Vec<String> = field_name
        .split('.')
        .map(|s| s.trim().to_string())
        .collect();
    trace!("Parsed simple field path: {:?}", parts);
    FieldPath::new(parts)
}

fn split_filter_template_manually(input: &str) -> Option<(&str, &str)> {
    // Look for patterns like "field == value {template}" or "field == value [template]"
    // Need to handle quotes and nested braces/brackets correctly

    // Track quote and bracket states
    let mut in_quotes = false;
    let mut quote_char = ' ';
    let mut brace_count = 0;
    let mut bracket_count = 0;
    let mut pos = 0;

    // Scan the input character by character
    let chars: Vec<char> = input.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '"' | '\'' => {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = c;
                } else if c == quote_char {
                    in_quotes = false;
                }
            }
            '{' => {
                if !in_quotes && brace_count == 0 && bracket_count == 0 && pos == 0 {
                    pos = i;
                }
                if !in_quotes {
                    brace_count += 1;
                }
            }
            '}' => {
                if !in_quotes {
                    brace_count -= 1;
                }
            }
            '[' => {
                if !in_quotes && brace_count == 0 && bracket_count == 0 && pos == 0 {
                    pos = i;
                }
                if !in_quotes {
                    bracket_count += 1;
                }
            }
            ']' => {
                if !in_quotes {
                    bracket_count -= 1;
                }
            }
            _ => {}
        }
    }

    // If we found a template start position
    if pos > 0 {
        let filter_part = input[..pos].trim();
        let template_part = input[pos..].trim();

        if !filter_part.is_empty() && !template_part.is_empty() {
            return Some((filter_part, template_part));
        }
    }

    None
}

fn parse_simple_filter(input: &str) -> Result<FilterExpr, Box<dyn std::error::Error>> {
    // Very basic filter parsing for common patterns
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.len() >= 3 {
        let field_name = parts[0];
        let operator = parts[1];
        let value_str = parts[2..].join(" ");

        let field_parts: Vec<String> = field_name.split('.').map(|s| s.to_string()).collect();
        let field = FieldPath::new(field_parts);

        let op = match operator {
            "==" => ComparisonOp::Equal,
            "!=" => ComparisonOp::NotEqual,
            ">" => ComparisonOp::GreaterThan,
            ">=" => ComparisonOp::GreaterThanOrEqual,
            "<" => ComparisonOp::LessThan,
            "<=" => ComparisonOp::LessThanOrEqual,
            "~" => ComparisonOp::Contains,
            "^=" => ComparisonOp::StartsWith,
            "$=" => ComparisonOp::EndsWith,
            "*=" => ComparisonOp::Contains,
            _ => return Err("Unknown operator".into()),
        };

        let value = if value_str.starts_with('"') && value_str.ends_with('"') {
            FilterValue::String(value_str[1..value_str.len() - 1].to_string())
        } else if value_str == "true" {
            FilterValue::Boolean(true)
        } else if value_str == "false" {
            FilterValue::Boolean(false)
        } else if value_str == "null" {
            FilterValue::Null
        } else if let Ok(num) = value_str.parse::<f64>() {
            FilterValue::Number(num)
        } else {
            FilterValue::String(value_str)
        };

        return Ok(FilterExpr::Comparison { field, op, value });
    }

    Err("Could not parse as simple filter".into())
}

/// Parse a field name into a FieldPath for truthy evaluation
fn parse_field_name_for_truthy(name: &str) -> FieldPath {
    // Strip the optional '?' suffix for boolean field indicators
    let base_name = name.strip_suffix('?').unwrap_or(name);
    trace!(
        "parse_field_name_for_truthy: '{}' -> base: '{}'",
        name, base_name
    );
    let parts: Vec<String> = base_name.split('.').map(|s| s.to_string()).collect();
    let field_path = FieldPath::new(parts);
    trace!(
        "parse_field_name_for_truthy: created FieldPath {:?}",
        field_path.parts
    );
    field_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{ComparisonOp, FilterExpr, FilterValue, TemplateItem};

    #[test]
    fn test_try_simple_template_patterns() {
        // Test braced template with variables
        let result = try_simple_template_patterns("{Hello ${name}}").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 2);

        // Test simple variable
        let result = try_simple_template_patterns("$name").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 1);
        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["name"]),
            _ => panic!("Expected field"),
        }

        // Test dollar amount (should be literal)
        let result = try_simple_template_patterns("$20").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 1);
        match &template.items[0] {
            TemplateItem::Literal(text) => assert_eq!(text, "$20"),
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn test_try_as_field_selector() {
        // Test simple field
        let result = try_as_field_selector("name").unwrap();
        assert!(result.field_selector.is_some());
        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["name"]);

        // Test nested field
        let result = try_as_field_selector("user.email").unwrap();
        assert!(result.field_selector.is_some());
        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["user", "email"]);

        // Test quoted field
        let result = try_as_field_selector("\"field with spaces\"").unwrap();
        assert!(result.field_selector.is_some());
        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["field with spaces"]);

        // Test single-quoted field
        let result = try_as_field_selector("'another field'").unwrap();
        assert!(result.field_selector.is_some());
        let field = result.field_selector.unwrap();
        assert_eq!(field.parts, vec!["another field"]);
    }

    #[test]
    fn test_try_boolean_with_truthy_fields() {
        // Test AND expression with truthy fields
        let result = try_boolean_with_truthy_fields("active? && verified?").unwrap();
        assert!(result.filter.is_some());
        match result.filter.unwrap() {
            FilterExpr::And(left, right) => match (left.as_ref(), right.as_ref()) {
                (FilterExpr::FieldTruthy(l), FilterExpr::FieldTruthy(r)) => {
                    assert_eq!(l.parts, vec!["active"]);
                    assert_eq!(r.parts, vec!["verified"]);
                }
                _ => panic!("Expected FieldTruthy expressions"),
            },
            _ => panic!("Expected AND expression"),
        }

        // Test OR expression with truthy fields
        let result = try_boolean_with_truthy_fields("premium? || admin?").unwrap();
        assert!(result.filter.is_some());
        match result.filter.unwrap() {
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
    fn test_try_parse_single_boolean_term() {
        // Test explicit truthy field
        let result = try_parse_single_boolean_term("active?").unwrap();
        match result {
            FilterExpr::FieldTruthy(field) => {
                assert_eq!(field.parts, vec!["active"]);
            }
            _ => panic!("Expected FieldTruthy"),
        }

        // Test NOT expression
        let result = try_parse_single_boolean_term("!suspended").unwrap();
        match result {
            FilterExpr::Not(inner) => match inner.as_ref() {
                FilterExpr::FieldTruthy(field) => {
                    assert_eq!(field.parts, vec!["suspended"]);
                }
                _ => panic!("Expected FieldTruthy inside NOT"),
            },
            _ => panic!("Expected NOT expression"),
        }

        // Test comparison expression
        let result = try_parse_single_boolean_term("age > 25").unwrap();
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
    fn test_try_parse_single_boolean_term_rejects_bare_fields() {
        // Bare field names should be rejected in boolean context
        assert!(try_parse_single_boolean_term("active").is_err());
        assert!(try_parse_single_boolean_term("user.verified").is_err());
        assert!(try_parse_single_boolean_term("field_name").is_err());
    }

    #[test]
    fn test_parse_field_name_for_truthy() {
        // Regular field
        let field = parse_field_name_for_truthy("active");
        assert_eq!(field.parts, vec!["active"]);

        // Field with ? suffix (should be stripped)
        let field = parse_field_name_for_truthy("active?");
        assert_eq!(field.parts, vec!["active"]);

        // Nested field
        let field = parse_field_name_for_truthy("user.verified");
        assert_eq!(field.parts, vec!["user", "verified"]);

        // Nested field with ? suffix
        let field = parse_field_name_for_truthy("user.verified?");
        assert_eq!(field.parts, vec!["user", "verified"]);
    }

    #[test]
    fn test_parse_simple_filter() {
        // Basic comparison
        let result = parse_simple_filter("age > 25").unwrap();
        match result {
            FilterExpr::Comparison { field, op, value } => {
                assert_eq!(field.parts, vec!["age"]);
                assert!(matches!(op, ComparisonOp::GreaterThan));
                assert!(matches!(value, FilterValue::Number(25.0)));
            }
            _ => panic!("Expected comparison"),
        }

        // String comparison
        let result = parse_simple_filter("name == \"Alice\"").unwrap();
        match result {
            FilterExpr::Comparison { field, op, value } => {
                assert_eq!(field.parts, vec!["name"]);
                assert!(matches!(op, ComparisonOp::Equal));
                assert!(matches!(value, FilterValue::String(s) if s == "Alice"));
            }
            _ => panic!("Expected comparison"),
        }

        // Boolean comparison
        let result = parse_simple_filter("active == true").unwrap();
        match result {
            FilterExpr::Comparison { field, op, value } => {
                assert_eq!(field.parts, vec!["active"]);
                assert!(matches!(op, ComparisonOp::Equal));
                assert!(matches!(value, FilterValue::Boolean(true)));
            }
            _ => panic!("Expected comparison"),
        }
    }

    #[test]
    fn test_split_filter_template_manually() {
        // Test filter + braced template
        let result = split_filter_template_manually("age > 25 {Hello ${name}}");
        assert!(result.is_some());
        let (filter_part, template_part) = result.unwrap();
        assert_eq!(filter_part, "age > 25");
        assert_eq!(template_part, "{Hello ${name}}");

        // Test filter + bracketed template
        let result = split_filter_template_manually("status == \"active\" [User: ${name}]");
        assert!(result.is_some());
        let (filter_part, template_part) = result.unwrap();
        assert_eq!(filter_part, "status == \"active\"");
        assert_eq!(template_part, "[User: ${name}]");

        // Test with quoted values containing braces (should not split)
        let result = split_filter_template_manually("message == \"Hello {world}\"");
        assert!(result.is_none());
    }

    #[test]
    fn test_try_fallback_parsing() {
        // Test successful template parsing
        let result = try_fallback_parsing("$name").unwrap();
        assert!(result.template.is_some());

        // Test successful field selector parsing
        let result = try_fallback_parsing("user.email").unwrap();
        assert!(result.field_selector.is_some());

        // Test successful boolean parsing
        let result = try_fallback_parsing("active? && verified?").unwrap();
        assert!(result.filter.is_some());

        // Test successful manual parsing of combined expression
        let result = try_fallback_parsing("age > 25 {Hello ${name}}").unwrap();
        assert!(result.filter.is_some());
        assert!(result.template.is_some());
    }

    #[test]
    fn test_edge_cases_and_error_handling() {
        // Test empty input to field selector
        assert!(try_as_field_selector("").is_err());

        // Test input with operators (should not be field selector)
        assert!(try_as_field_selector("age > 25").is_err());
        assert!(try_as_field_selector("name == value").is_err());

        // Test boolean expressions without operators
        assert!(try_boolean_with_truthy_fields("just_a_field").is_err());

        // Test incomplete filter
        assert!(parse_simple_filter("age >").is_err());
        assert!(parse_simple_filter("name").is_err());
    }

    #[test]
    fn test_numeric_vs_variable_distinction() {
        // Test ${0} special case (should map to $0)
        let result = try_simple_template_patterns("${0}").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["$0"]),
            _ => panic!("Expected ${{0}} to map to $0 field"),
        }

        // Test ${1} (should map to "1")
        let result = try_simple_template_patterns("${1}").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["1"]),
            _ => panic!("Expected ${{1}} to map to '1' field"),
        }

        // Test $1 (should be literal dollar amount)
        let result = try_simple_template_patterns("$1").unwrap();
        assert!(result.template.is_some());
        let template = result.template.unwrap();
        match &template.items[0] {
            TemplateItem::Literal(text) => assert_eq!(text, "$1"),
            _ => panic!("Expected $1 to be literal"),
        }
    }
}
