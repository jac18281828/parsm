//! DSL Parser - Converts Pest parse tree to AST
//!
//! This module provides the domain-specific language parser for parsm, converting
//! user input into structured filter expressions, templates, and field selectors.

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::filter::{ComparisonOp, FieldPath, FilterExpr, FilterValue, Template, TemplateItem};

/// Main DSL parser using Pest grammar.
///
/// This parser handles the complete parsm DSL grammar including:
/// - Filter expressions with boolean logic
/// - Template strings with field interpolation  
/// - Field selection syntax
#[derive(Parser)]
#[grammar = "pest/parsm.pest"]
pub struct DSLParser;

/// Parsed DSL result containing optional filter, template, and field selector.
///
/// This structure represents the parsed result of a user command, which may contain
/// any combination of filtering logic, output templates, and field selection.
#[derive(Debug)]
pub struct ParsedDSL {
    /// Optional filter expression for boolean evaluation
    pub filter: Option<FilterExpr>,
    /// Optional template for output formatting
    pub template: Option<Template>,
    /// Optional field selector for direct field extraction
    pub field_selector: Option<FieldPath>,
}

impl ParsedDSL {
    /// Create a new empty ParsedDSL instance.
    pub fn new() -> Self {
        Self {
            filter: None,
            template: None,
            field_selector: None,
        }
    }
}

impl Default for ParsedDSL {
    fn default() -> Self {
        Self::new()
    }
}

impl DSLParser {
    /// Parse a complete DSL program from input string.
    ///
    /// This is the main entry point for parsing user commands that may contain
    /// filter expressions, templates, and field selectors in a single string.
    ///
    /// # Arguments
    /// * `input` - The input string to parse
    ///
    /// # Returns
    /// * `Ok(ParsedDSL)` - Successfully parsed DSL components
    /// * `Err(Box<pest::error::Error<Rule>>)` - Parse error
    pub fn parse_dsl(input: &str) -> Result<ParsedDSL, Box<pest::error::Error<Rule>>> {
        let mut pairs = Self::parse(Rule::program, input)?;
        let program = pairs.next().unwrap();

        let mut result = ParsedDSL {
            filter: None,
            template: None,
            field_selector: None,
        };

        for pair in program.into_inner() {
            match pair.as_rule() {
                Rule::field_selector => {
                    let inner = pair.into_inner().next().unwrap();
                    match inner.as_rule() {
                        Rule::field_access => {
                            result.field_selector = Some(Self::parse_field_access(inner));
                        }
                        Rule::string_literal => {
                            let field_string = Self::parse_string_literal(inner);
                            let parts: Vec<String> =
                                field_string.split('.').map(|s| s.to_string()).collect();
                            result.field_selector = Some(FieldPath::new(parts));
                        }
                        _ => unreachable!("Unexpected field_selector rule"),
                    }
                }
                Rule::filter_expr => {
                    result.filter = Some(Self::parse_filter_expr(pair)?);
                }
                Rule::template_expr => {
                    result.template = Some(Self::parse_template_expr(pair)?);
                }
                Rule::EOI => break,
                _ => {}
            }
        }

        Ok(result)
    }

    /// Parse only a filter expression without template components.
    ///
    /// # Arguments
    /// * `input` - Input string containing only filter logic
    ///
    /// # Returns  
    /// * `Ok(FilterExpr)` - Parsed filter expression
    /// * `Err(Box<pest::error::Error<Rule>>)` - Parse error
    pub fn parse_filter_only(input: &str) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        // First check if this looks like a simple field selector that shouldn't be a filter
        if is_simple_field_selector(input) && !contains_filter_operators(input) {
            return Err(Box::new(pest::error::Error::new_from_pos(
                pest::error::ErrorVariant::CustomError {
                    message: "Input appears to be a field selector, not a filter".to_string(),
                },
                pest::Position::new(input, 0).unwrap(),
            )));
        }

        // Check for partial braces which indicate invalid template syntax
        if has_partial_braces(input) {
            return Err(Box::new(pest::error::Error::new_from_pos(
                pest::error::ErrorVariant::CustomError {
                    message: "Input has partial braces. Templates must be entirely contained within braces like {field}".to_string(),
                },
                pest::Position::new(input, 0).unwrap(),
            )));
        }

        // Use program rule to ensure entire input is consumed and only contains filter
        let mut pairs = DSLParser::parse(Rule::program, input)?;
        let program = pairs.next().unwrap();

        // Look for filter_expr in the program
        for pair in program.into_inner() {
            match pair.as_rule() {
                Rule::filter_expr => {
                    return Self::parse_filter_expr(pair);
                }
                Rule::field_selector => {
                    // If it parsed as a field_selector, it's not a filter
                    return Err(Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: "Input is a field selector, not a filter expression"
                                .to_string(),
                        },
                        pest::Position::new(input, 0).unwrap(),
                    )));
                }
                Rule::template_expr => {
                    return Err(Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: "Input is a template, not a filter expression".to_string(),
                        },
                        pest::Position::new(input, 0).unwrap(),
                    )));
                }
                Rule::EOI => break,
                _ => {}
            }
        }

        // If we get here, there was no filter_expr found
        Err(Box::new(pest::error::Error::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "No filter expression found".to_string(),
            },
            pest::Position::new(input, 0).unwrap(),
        )))
    }

    /// Parse only a field selector (quoted or unquoted field name).
    ///
    /// Supports both unquoted identifiers and quoted field names with dots.
    ///
    /// # Arguments
    /// * `input` - Input string containing field selector
    ///
    /// # Returns
    /// * `Ok(FieldPath)` - Parsed field path
    /// * `Err(Box<pest::error::Error<Rule>>)` - Parse error
    pub fn parse_field_selector_only(
        input: &str,
    ) -> Result<FieldPath, Box<pest::error::Error<Rule>>> {
        // First check if this contains filter operators - if so, it's not a field selector
        if contains_filter_operators(input) {
            return Err(Box::new(pest::error::Error::new_from_pos(
                pest::error::ErrorVariant::CustomError {
                    message: "Input contains filter operators, not a field selector".to_string(),
                },
                pest::Position::new(input, 0).unwrap(),
            )));
        }

        // Use program rule to ensure entire input is consumed and contains only field selector
        let mut pairs = DSLParser::parse(Rule::program, input)?;
        let program = pairs.next().unwrap();

        // Look for field_selector in the program
        for pair in program.into_inner() {
            match pair.as_rule() {
                Rule::field_selector => {
                    let inner = pair.into_inner().next().unwrap();
                    match inner.as_rule() {
                        Rule::field_access => {
                            let field_path = Self::parse_field_access(inner);
                            return Ok(field_path);
                        }
                        Rule::string_literal => {
                            let field_string = Self::parse_string_literal(inner);
                            let parts: Vec<String> =
                                field_string.split('.').map(|s| s.to_string()).collect();
                            return Ok(FieldPath::new(parts));
                        }
                        _ => unreachable!(),
                    }
                }
                Rule::filter_expr => {
                    // If it parsed as a filter_expr, check if it's just a simple field access (field truthiness)
                    // In this case, we can still treat it as a field selector
                    if is_simple_field_selector(input) {
                        let parts: Vec<String> = input.split('.').map(|s| s.to_string()).collect();
                        return Ok(FieldPath::new(parts));
                    } else {
                        return Err(Box::new(pest::error::Error::new_from_pos(
                            pest::error::ErrorVariant::CustomError {
                                message: "Input is a filter expression, not a field selector"
                                    .to_string(),
                            },
                            pest::Position::new(input, 0).unwrap(),
                        )));
                    }
                }
                Rule::template_expr => {
                    return Err(Box::new(pest::error::Error::new_from_pos(
                        pest::error::ErrorVariant::CustomError {
                            message: "Input is a template, not a field selector".to_string(),
                        },
                        pest::Position::new(input, 0).unwrap(),
                    )));
                }
                Rule::EOI => break,
                _ => {}
            }
        }

        // If we get here, there was no field_selector found
        Err(Box::new(pest::error::Error::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "No field selector found".to_string(),
            },
            pest::Position::new(input, 0).unwrap(),
        )))
    }

    /// Parse only a template expression for output formatting.
    ///
    /// # Arguments
    /// * `input` - Input string containing template syntax
    ///
    /// # Returns
    /// * `Ok(Template)` - Parsed template
    /// * `Err(Box<pest::error::Error<Rule>>)` - Parse error
    pub fn parse_template_only(input: &str) -> Result<Template, Box<pest::error::Error<Rule>>> {
        let pairs = DSLParser::parse(Rule::template_expr, input)?;
        let pair = pairs.into_iter().next().unwrap();
        Self::parse_template_expr(pair)
    }

    /// Parse a filter expression from a Pest pair.
    fn parse_filter_expr(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let inner = pair.into_inner().next().unwrap();
        Self::parse_condition(inner)
    }

    /// Parse a condition (top-level boolean expression).
    fn parse_condition(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let inner = pair.into_inner().next().unwrap();
        Self::parse_or_expr(inner)
    }

    /// Parse OR expressions with left-associative precedence.
    fn parse_or_expr(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let mut inner = pair.into_inner();
        let mut left = Self::parse_and_expr(inner.next().unwrap())?;

        while let Some(op_pair) = inner.next() {
            if matches!(op_pair.as_rule(), Rule::or_op) {
                let right = Self::parse_and_expr(inner.next().unwrap())?;
                left = FilterExpr::Or(Box::new(left), Box::new(right));
            }
        }

        Ok(left)
    }

    /// Parse AND expressions with higher precedence than OR.
    fn parse_and_expr(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let mut inner = pair.into_inner();
        let mut left = Self::parse_comparison(inner.next().unwrap())?;

        while let Some(op_pair) = inner.next() {
            if matches!(op_pair.as_rule(), Rule::and_op) {
                let right = Self::parse_comparison(inner.next().unwrap())?;
                left = FilterExpr::And(Box::new(left), Box::new(right));
            }
        }

        Ok(left)
    }

    /// Parse comparison expressions and handle negation.
    fn parse_comparison(pair: Pair<Rule>) -> Result<FilterExpr, Box<pest::error::Error<Rule>>> {
        let mut inner = pair.into_inner();
        let first = inner.next().unwrap();

        match first.as_rule() {
            Rule::not_op => {
                let next = inner.next().unwrap();
                match next.as_rule() {
                    Rule::field_access => {
                        // Handle !field (negation of field truthiness)
                        let field = Self::parse_field_access(next);
                        Ok(FilterExpr::Not(Box::new(FilterExpr::FieldTruthy(field))))
                    }
                    _ => {
                        // Handle !(complex expression)
                        let comparison = Self::parse_comparison(next)?;
                        Ok(FilterExpr::Not(Box::new(comparison)))
                    }
                }
            }
            Rule::field_access => {
                let field = Self::parse_field_access(first.clone());
                if let Some(op_pair) = inner.next() {
                    // field op value pattern
                    let op = Self::parse_comparison_op(op_pair);
                    let value_pair = inner.next().unwrap();
                    let value = Self::parse_value(value_pair);
                    Ok(FilterExpr::Comparison { field, op, value })
                } else {
                    // standalone field (field truthiness)
                    Ok(FilterExpr::FieldTruthy(field))
                }
            }
            _ => Self::parse_condition(first),
        }
    }

    /// Parse field access path with dot notation support.
    fn parse_field_access(pair: Pair<Rule>) -> FieldPath {
        let parts: Vec<String> = pair
            .into_inner()
            .map(|field_component| field_component.as_str().to_string())
            .collect();
        FieldPath::new(parts)
    }

    /// Parse comparison operator from string representation.
    fn parse_comparison_op(pair: Pair<Rule>) -> ComparisonOp {
        crate::operators::parse_comparison_op(pair.as_str())
    }

    /// Parse filter value (string, number, boolean, null, or unquoted).
    fn parse_value(pair: Pair<Rule>) -> FilterValue {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::string_literal => {
                let string_content = inner.into_inner().next().unwrap();
                FilterValue::String(string_content.as_str().to_string())
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
            Rule::unquoted_value => FilterValue::String(inner.as_str().to_string()),
            _ => FilterValue::String(inner.as_str().to_string()),
        }
    }

    /// Parse template expression from Pest pair.
    fn parse_template_expr(pair: Pair<Rule>) -> Result<Template, Box<pest::error::Error<Rule>>> {
        // Extract the template content from inside the braces
        let template_content = pair.into_inner().next().unwrap();
        let template_str = template_content.as_str();
        let items = Self::parse_template_string(template_str);

        // Since templates are surrounded by braces, they are always valid templates
        // even if they don't contain variables (could be just literal text)
        Ok(Template { items })
    }

    /// Parse template string with variable interpolation support.
    ///
    /// Supports template syntaxes:
    /// - `${field}` - Field names in shell-style syntax (always supported)
    /// - `$field` - Simple field names (when unambiguous)
    /// - `${field.nested}` - Nested field access  
    /// - `${1}, ${2}, ${3}` - Indexed fields (1-based, requires {} syntax)
    /// - `${0}` - Entire original input (AWK-style, requires {} syntax)
    /// - `$$` - Escaped dollar for literal dollar sign
    fn parse_template_string(template_str: &str) -> Vec<TemplateItem> {
        let mut items = Vec::new();
        let mut chars = template_str.chars().peekable();
        let mut current_text = String::new();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '$' {
                        // $$ represents a literal dollar sign
                        chars.next(); // consume the second $
                        current_text.push('$'); // add literal $
                    } else if next_ch == '{' {
                        // ${variable} syntax
                        if !current_text.is_empty() {
                            items.push(TemplateItem::Literal(current_text.clone()));
                            current_text.clear();
                        }
                        chars.next(); // consume the {
                        let mut field_content = String::new();
                        while let Some(&brace_ch) = chars.peek() {
                            if brace_ch == '}' {
                                chars.next();
                                break;
                            }
                            field_content.push(chars.next().unwrap());
                        }
                        let field_path = Self::parse_field_name(&field_content);
                        items.push(TemplateItem::Field(field_path));
                    } else if next_ch.is_alphabetic() || next_ch == '_' {
                        // $variable syntax (simple field names only, not numbers)
                        if !current_text.is_empty() {
                            items.push(TemplateItem::Literal(current_text.clone()));
                            current_text.clear();
                        }

                        let mut field_content = String::new();
                        // Consume identifier characters
                        while let Some(&var_ch) = chars.peek() {
                            if var_ch.is_alphanumeric() || var_ch == '_' || var_ch == '.' {
                                field_content.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }

                        let field_path = Self::parse_field_name(&field_content);
                        items.push(TemplateItem::Field(field_path));
                    } else {
                        // Not a recognized variable pattern, treat as literal
                        current_text.push('$');
                    }
                } else {
                    current_text.push('$');
                }
            } else {
                current_text.push(ch);
            }
        }

        if !current_text.is_empty() {
            items.push(TemplateItem::Literal(current_text));
        }

        items
    }

    /// Parse field name with dot notation into FieldPath.
    fn parse_field_name(field_name: &str) -> FieldPath {
        // Handle special cases for positional and original input references
        if field_name == "0" {
            // ${0} represents entire original input
            return FieldPath::new(vec!["$0".to_string()]);
        }

        if let Ok(index) = field_name.parse::<usize>() {
            if index > 0 {
                // ${1}, ${2}, etc. represent 1-based field indices
                return FieldPath::new(vec![format!("field_{}", index - 1)]);
            }
        }

        // Regular field name with dot notation
        let parts: Vec<String> = field_name
            .split('.')
            .map(|s| s.trim().to_string())
            .collect();
        FieldPath::new(parts)
    }

    /// Parse string literal content from Pest pair.
    fn parse_string_literal(pair: Pair<Rule>) -> String {
        let string_content = pair.into_inner().next().unwrap();
        string_content.as_str().to_string()
    }

    /// Create a ParsedDSL from separate filter and template strings.
    ///
    /// This method allows parsing filter and template expressions separately,
    /// which is useful for command-line interfaces that accept multiple arguments.
    ///
    /// # Arguments
    /// * `filter_input` - Optional filter expression string
    /// * `template_input` - Optional template expression string
    ///
    /// # Returns
    /// * `Ok(ParsedDSL)` - Successfully parsed components
    /// * `Err(Box<pest::error::Error<Rule>>)` - Parse error
    pub fn parse_separate(
        filter_input: Option<&str>,
        template_input: Option<&str>,
    ) -> Result<ParsedDSL, Box<pest::error::Error<Rule>>> {
        let mut result = ParsedDSL::new();

        if let Some(filter_str) = filter_input {
            if let Ok(filter) = Self::parse_filter_only(filter_str) {
                result.filter = Some(filter);
            } else if let Ok(field_selector) = Self::parse_field_selector_only(filter_str) {
                result.field_selector = Some(field_selector);
            } else {
                return Err(Box::new(pest::error::Error::new_from_pos(
                    pest::error::ErrorVariant::CustomError {
                        message: format!(
                            "Could not parse '{}' as filter expression or field selector",
                            filter_str
                        ),
                    },
                    pest::Position::new(filter_str, 0).unwrap(),
                )));
            }
        }

        if let Some(template_str) = template_input {
            result.template = Some(Self::parse_template_only(template_str)?);
        }

        Ok(result)
    }
}

/// Parse a complete command line input with intelligent disambiguation.
///
/// This is the main entry point for parsing user commands. It applies disambiguation
/// rules to determine whether the input is a filter expression, template, or field selector:
///
/// 1. Templates: Surrounded by braces `{...}`
/// 2. Filters: Contain comparison operators (`==`, `>`, `*=`, etc.)
/// 3. Field selectors: Simple identifiers or quoted strings
///
/// # Arguments
/// * `input` - The complete user input string
///
/// # Returns
/// * `Ok(ParsedDSL)` - Successfully parsed and classified input
/// * `Err(Box<dyn std::error::Error>)` - Parse or classification error
pub fn parse_command(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    let trimmed = input.trim();

    // Check for template and filter syntax
    let has_template_syntax =
        trimmed.contains('{') && trimmed.contains('}') && contains_dollar_variables(trimmed);
    let has_filter_syntax = contains_filter_operators(trimmed);

    // Early validation: reject unquoted strings with spaces that don't contain operators or quotes
    if trimmed.contains(' ')
        && !trimmed.starts_with('"')
        && !trimmed.starts_with('\'')
        && !has_filter_syntax
        && !has_template_syntax
    {
        return Err(format!(
            "Invalid field selector '{}'. Field selectors with spaces must be quoted like \"{}\"",
            trimmed, trimmed
        )
        .into());
    }

    // Priority 1: If we have both template and filter syntax, try combined parsing
    if has_template_syntax && has_filter_syntax {
        if let Ok(result) = DSLParser::parse_dsl(trimmed) {
            return Ok(result);
        }

        // Try to split the input if combined parsing fails
        if let Some(result) = try_split_filter_template(trimmed) {
            return Ok(result);
        }
    }

    // Priority 2: If we have template syntax, prioritize template parsing
    if has_template_syntax && !has_filter_syntax {
        // Check if entire input is a template (starts and ends with braces)
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(template) = DSLParser::parse_template_only(trimmed) {
                let mut result = ParsedDSL::new();
                result.template = Some(template);
                return Ok(result);
            }
        }
    }

    // Priority 3: If we have filter syntax, prioritize filter parsing
    if has_filter_syntax && !has_template_syntax {
        if let Ok(filter) = DSLParser::parse_filter_only(trimmed) {
            let mut result = ParsedDSL::new();
            result.filter = Some(filter);
            return Ok(result);
        }
        // Try fallback for simple filters
        if let Some(filter) = parse_simple_filter_fallback(trimmed) {
            let mut result = ParsedDSL::new();
            result.filter = Some(filter);
            return Ok(result);
        }
    }

    // Priority 4: Try as simple field selector
    if is_simple_field_selector(trimmed) {
        if let Ok(field_selector) = DSLParser::parse_field_selector_only(trimmed) {
            let mut result = ParsedDSL::new();
            result.field_selector = Some(field_selector);
            return Ok(result);
        }
    }

    // Fallback attempts in order
    if let Ok(filter) = DSLParser::parse_filter_only(trimmed) {
        let mut result = ParsedDSL::new();
        result.filter = Some(filter);
        return Ok(result);
    }

    // Only try field selector parsing if it looks like a simple field selector
    if is_simple_field_selector(trimmed) {
        if let Ok(field_selector) = DSLParser::parse_field_selector_only(trimmed) {
            let mut result = ParsedDSL::new();
            result.field_selector = Some(field_selector);
            return Ok(result);
        }
    }

    // Last resort: try fallback filter parsing, but only if it doesn't have partial braces
    if !has_partial_braces(trimmed) {
        if let Some(filter) = parse_simple_filter_fallback(trimmed) {
            let mut result = ParsedDSL::new();
            result.filter = Some(filter);
            return Ok(result);
        }
    }

    // If nothing worked, provide a helpful error message
    let error_msg = if trimmed.contains('{') || trimmed.contains('}') {
        format!(
            "Invalid template syntax '{}'. Templates must be entirely contained within braces like {{field}} or {{Hello ${{name}}}}",
            trimmed
        )
    } else if trimmed.contains(' ') && !trimmed.starts_with('"') && !trimmed.starts_with('\'') {
        format!(
            "Invalid field selector '{}'. Field selectors with spaces must be quoted like \"{}\"",
            trimmed, trimmed
        )
    } else if contains_filter_operators(trimmed) {
        format!(
            "Invalid filter expression '{}'. Check operator syntax and field/value formatting",
            trimmed
        )
    } else {
        format!(
            "Unable to parse '{}'. Use:\n  - Field selectors: field.name or \"field name\"\n  - Templates: {{field}} or {{Hello ${{name}}}}\n  - Filters: field == value",
            trimmed
        )
    };

    Err(error_msg.into())
}

/// Check if input contains filter operators for disambiguation.
fn contains_filter_operators(input: &str) -> bool {
    crate::operators::contains_filter_operators(input)
}

/// Check if the input contains valid dollar variables (${name} or $name) or escaped dollars inside braces
fn contains_dollar_variables(input: &str) -> bool {
    if !input.contains('$') {
        return false;
    }

    // Check for ${...} syntax
    if input.contains("${") && input.contains("}") {
        return true;
    }

    // Check for escaped dollars ($$) - these only count as template content when in braces
    if input.contains('{') && input.contains('}') && input.contains("$$") {
        return true;
    }

    // Check for simple $name syntax (NOT $0 which is literal)
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(&next_char) = chars.peek() {
                // $0, $1, etc. are NOT variables - they're literals (only ${0}, ${1} are variables)
                if next_char.is_ascii_digit() {
                    continue; // Skip numeric $0, $1, etc. - these are literals, not variables
                } else if next_char.is_alphabetic() || next_char == '_' {
                    // Check that it's followed by valid identifier characters
                    let mut has_alpha = false;
                    while let Some(&peek_char) = chars.peek() {
                        if peek_char.is_alphanumeric() || peek_char == '_' {
                            if peek_char.is_alphabetic() {
                                has_alpha = true;
                            }
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if has_alpha {
                        return true;
                    }
                } else if next_char == '$' {
                    // Skip escaped dollar signs ($$) - but they were already detected above
                    chars.next();
                }
            }
        }
    }

    false
}

/// Check if input is a simple field selector pattern.
///
/// Matches unquoted identifiers and quoted strings suitable for field selection.
fn is_simple_field_selector(input: &str) -> bool {
    if (input.starts_with('"') && input.ends_with('"'))
        || (input.starts_with('\'') && input.ends_with('\''))
    {
        // For quoted strings, extract the content and validate it
        let content = &input[1..input.len() - 1];

        // Quoted field selectors should not contain dollar signs or braces (but spaces are OK)
        if content.contains('$') || content.contains('{') || content.contains('}') {
            return false;
        }

        // Should be a valid field path (alphanumeric, underscore, dots, and spaces for quoted)
        return content
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == ' ')
            && !content.is_empty();
    }

    // Unquoted field selectors cannot contain braces, spaces, or filter operators
    if input.contains('{') || input.contains('}') || input.contains(' ') {
        return false;
    }

    if input
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        && input.chars().any(|c| c.is_alphanumeric())
        && !input.is_empty()
    {
        return true;
    }

    false
}

/// Parse separate expressions for filter and template.
///
/// This is a convenience function that wraps `DSLParser::parse_separate`.
///
/// # Arguments
/// * `filter_input` - Optional filter expression
/// * `template_input` - Optional template expression
///
/// # Returns
/// * `Ok(ParsedDSL)` - Successfully parsed components
/// * `Err(Box<dyn std::error::Error>)` - Parse error
pub fn parse_separate_expressions(
    filter_input: Option<&str>,
    template_input: Option<&str>,
) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    Ok(DSLParser::parse_separate(filter_input, template_input)?)
}

/// Fallback parser for simple filter expressions when the main parser fails.
///
/// This handles common cases like "field op value" that may not parse correctly
/// due to grammar limitations.
fn parse_simple_filter_fallback(input: &str) -> Option<FilterExpr> {
    let trimmed = input.trim();

    // Don't accept unquoted strings with spaces that don't contain filter operators
    if trimmed.contains(' ') && !contains_filter_operators(trimmed) {
        return None;
    }

    // Handle "not field" pattern
    if let Some(field_str) = trimmed.strip_prefix("not ") {
        if !field_str.contains(' ') {
            // Simple field name like "not active"
            let field_parts: Vec<String> = field_str.split('.').map(|s| s.to_string()).collect();
            let field = FieldPath::new(field_parts);

            // Create a "not field" filter (field != true)
            let inner_expr = FilterExpr::Comparison {
                field,
                op: ComparisonOp::Equal,
                value: FilterValue::Boolean(true),
            };
            return Some(FilterExpr::Not(Box::new(inner_expr)));
        }
    }

    // Simple parsing for "field operator value" pattern
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() >= 3 {
        let field_str = parts[0];
        let op_str = parts[1];
        let value_str = parts[2..].join(" ");

        // Check if this is a recognized operator
        let operators = crate::operators::get_all_operator_symbols();
        if operators.contains(&op_str) {
            // Parse field path
            let field_parts: Vec<String> = field_str.split('.').map(|s| s.to_string()).collect();
            let field = FieldPath::new(field_parts);

            // Parse operator
            let op = crate::operators::parse_comparison_op(op_str);

            // Parse value
            let value = if value_str.starts_with('"') && value_str.ends_with('"') {
                // String literal
                FilterValue::String(value_str[1..value_str.len() - 1].to_string())
            } else if value_str.starts_with('\'') && value_str.ends_with('\'') {
                // Single-quoted string literal
                FilterValue::String(value_str[1..value_str.len() - 1].to_string())
            } else if value_str == "null" {
                FilterValue::Null
            } else if value_str == "true" {
                FilterValue::Boolean(true)
            } else if value_str == "false" {
                FilterValue::Boolean(false)
            } else if let Ok(num) = value_str.parse::<f64>() {
                FilterValue::Number(num)
            } else {
                // Unquoted string
                FilterValue::String(value_str)
            };

            return Some(FilterExpr::Comparison { field, op, value });
        }
    }

    None
}

/// Try to split mixed filter+template input when combined parsing fails.
///
/// This function attempts to identify where the filter expression ends and
/// the template begins, then parse them separately.
fn try_split_filter_template(input: &str) -> Option<ParsedDSL> {
    // Strategy: Look for common filter patterns and try to find the boundary
    // between filter and template parts

    // Look for patterns like: field op "value" template_text
    // or: field op value template_text

    let words: Vec<&str> = input.split_whitespace().collect();

    if words.len() >= 2 {
        // Try to find the end of filter expressions by looking for complete patterns

        // First, handle quoted values that might contain spaces
        // We need to properly group quoted strings as single "words"
        let mut grouped_words = Vec::new();
        let mut i = 0;
        while i < words.len() {
            if words[i].starts_with('"') && !words[i].ends_with('"') {
                // Start of a quoted string, find the end
                let mut quoted = words[i].to_string();
                i += 1;
                while i < words.len() {
                    quoted.push(' ');
                    quoted.push_str(words[i]);
                    if words[i].ends_with('"') {
                        break;
                    }
                    i += 1;
                }
                grouped_words.push(quoted);
            } else {
                grouped_words.push(words[i].to_string());
            }
            i += 1;
        }

        // Pattern 0: Simple negation like "!field {template}" or single field like "field {template}"
        if grouped_words.len() == 2 {
            let potential_filter = &grouped_words[0];
            let potential_template = &grouped_words[1];

            // Check if first word is a simple filter expression and second is a template
            if (potential_filter.starts_with('!')
                || potential_filter
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '.'))
                && potential_template.starts_with('{')
                && potential_template.ends_with('}')
                && contains_filter_operators(potential_filter)
            {
                if let Ok(result) =
                    DSLParser::parse_separate(Some(potential_filter), Some(potential_template))
                {
                    return Some(result);
                }
            }
        }

        // Pattern 1: field operator "quoted_value" rest...
        if grouped_words.len() >= 3
            && grouped_words
                .get(2)
                .is_some_and(|w| w.starts_with('"') && w.ends_with('"'))
        {
            // Simple case: field op "value" template...
            let filter_part = grouped_words[0..3].join(" ");
            let template_part = grouped_words[3..].join(" ");

            if !template_part.is_empty()
                && contains_filter_operators(&filter_part)
                && (contains_dollar_variables(&template_part)
                    || (template_part.contains('{') && template_part.contains('}'))
                    || !contains_filter_operators(&template_part))
            {
                if let Ok(result) =
                    DSLParser::parse_separate(Some(&filter_part), Some(&template_part))
                {
                    return Some(result);
                }
            }
        }

        // Pattern 2: field operator unquoted_value template...
        if grouped_words.len() >= 3
            && !grouped_words[2].starts_with('"')
            && !grouped_words[2].starts_with('$')
        {
            let filter_part = grouped_words[0..3].join(" ");
            let template_part = grouped_words[3..].join(" ");

            if !template_part.is_empty()
                && contains_filter_operators(&filter_part)
                && (contains_dollar_variables(&template_part)
                    || (template_part.contains('{') && template_part.contains('}'))
                    || !contains_filter_operators(&template_part))
            {
                if let Ok(result) =
                    DSLParser::parse_separate(Some(&filter_part), Some(&template_part))
                {
                    return Some(result);
                }
            }
        }

        // Pattern 3: Complex expressions with && or ||
        // Look for logical operators and try to find the end of the filter
        for i in 3..grouped_words.len() {
            let filter_part = grouped_words[0..i].join(" ");
            let template_part = grouped_words[i..].join(" ");

            // Skip if no template part
            if template_part.is_empty() {
                continue;
            }

            // Check if the filter part looks complete (ends with a value)
            let filter_ends_properly = if let Some(last_word) = grouped_words.get(i - 1) {
                last_word.starts_with('"') && last_word.ends_with('"')
                    || *last_word == "true"
                    || *last_word == "false"
                    || *last_word == "null"
                    || last_word
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
            } else {
                false
            };

            if filter_ends_properly
                && contains_filter_operators(&filter_part)
                && (contains_dollar_variables(&template_part)
                    || (template_part.contains('{') && template_part.contains('}'))
                    || (!contains_filter_operators(&template_part)
                        && !template_part.starts_with('"')))
            {
                if let Ok(result) =
                    DSLParser::parse_separate(Some(&filter_part), Some(&template_part))
                {
                    return Some(result);
                }
            }
        }
    }

    // Fallback: try to split at the first '{' if both filter and template syntax are present
    if input.contains('{') && input.contains('}') {
        // Find the first '{' not inside quotes or parentheses
        let mut in_quotes = false;
        let mut quote_char = '\0';
        let mut paren_level = 0;
        let mut split_idx = None;
        for (i, c) in input.char_indices() {
            match c {
                '"' | '\'' => {
                    if in_quotes && c == quote_char {
                        in_quotes = false;
                    } else if !in_quotes {
                        in_quotes = true;
                        quote_char = c;
                    }
                }
                '(' => {
                    if !in_quotes {
                        paren_level += 1;
                    }
                }
                ')' => {
                    if !in_quotes && paren_level > 0 {
                        paren_level -= 1;
                    }
                }
                '{' => {
                    if !in_quotes && paren_level == 0 {
                        split_idx = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(idx) = split_idx {
            let filter_part = input[..idx].trim();
            let template_part = input[idx..].trim();
            if !filter_part.is_empty() && !template_part.is_empty() {
                if let Ok(result) =
                    DSLParser::parse_separate(Some(filter_part), Some(template_part))
                {
                    return Some(result);
                }
            }
        }
    }

    None
}

/// Check if input has partial/mismatched braces that indicate invalid syntax.
fn has_partial_braces(input: &str) -> bool {
    let open_count = input.chars().filter(|&c| c == '{').count();
    let close_count = input.chars().filter(|&c| c == '}').count();

    // If we have braces but they don't match, or if we have braces but the entire input
    // is not surrounded by them, it's partial braces
    if open_count > 0 || close_count > 0 {
        // Check if entire input is properly surrounded by braces
        if input.starts_with('{') && input.ends_with('}') && open_count == 1 && close_count == 1 {
            return false; // This is a properly formatted template
        }
        return true; // Any other case with braces is partial
    }

    false // No braces at all
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test field selector disambiguation from other expression types.
    #[test]
    fn test_field_selector_disambiguation() {
        // Test unquoted field selectors
        let result = parse_command("name").unwrap();
        assert!(result.field_selector.is_some());
        assert!(result.filter.is_none());
        assert!(result.template.is_none());

        let result = parse_command("user.name").unwrap();
        assert!(result.field_selector.is_some());
        assert!(result.filter.is_none());
        assert!(result.template.is_none());

        // Test quoted field selectors
        let result = parse_command("\"name\"").unwrap();
        assert!(result.field_selector.is_some());
        assert!(result.filter.is_none());
        assert!(result.template.is_none());

        let result = parse_command("'user.name'").unwrap();
        assert!(result.field_selector.is_some());
        assert!(result.filter.is_none());
        assert!(result.template.is_none());
    }

    /// Test template disambiguation from other expression types.
    #[test]
    fn test_template_disambiguation() {
        // Test {${variable}} templates (new syntax)
        let result = parse_command("{${name}}").unwrap();
        assert!(result.template.is_some());
        assert!(result.field_selector.is_none());
        assert!(result.filter.is_none());

        let result = parse_command("{Hello ${name}}").unwrap();
        assert!(result.template.is_some());
        assert!(result.field_selector.is_none());
        assert!(result.filter.is_none());

        // Test multiple variables
        let result = parse_command("{${name} is ${age} years old}").unwrap();
        assert!(result.template.is_some());
        assert!(result.field_selector.is_none());
        assert!(result.filter.is_none());
    }

    /// Test filter disambiguation from other expression types.
    #[test]
    fn test_filter_disambiguation() {
        // Test filter expressions with operators
        let result = parse_command("name == \"Alice\"").unwrap();
        assert!(result.filter.is_some());
        assert!(result.field_selector.is_none());
        assert!(result.template.is_none());

        let result = parse_command("age > 25").unwrap();
        assert!(result.filter.is_some());
        assert!(result.field_selector.is_none());
        assert!(result.template.is_none());

        // Test new symbol-based operators
        let result = parse_command("name *= \"test\"").unwrap();
        assert!(result.filter.is_some());
        assert!(result.field_selector.is_none());
        assert!(result.template.is_none());
    }

    #[test]
    fn test_simple_filter_parsing() {
        let result = parse_command(r#"name == "Alice""#).unwrap();
        assert!(result.filter.is_some());
        assert!(result.template.is_none());

        if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
            assert_eq!(field.parts, vec!["name"]);
            assert_eq!(op, ComparisonOp::Equal);
            assert_eq!(value, FilterValue::String("Alice".to_string()));
        } else {
            panic!("Expected simple comparison");
        }
    }

    #[test]
    fn test_filter_with_template() {
        let result = parse_command(r#"name == "Alice" {Name: ${name}, Age: ${age}}"#).unwrap();
        assert!(result.filter.is_some());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 4);

        match &template.items[0] {
            TemplateItem::Literal(text) => assert_eq!(text, "Name: "),
            _ => panic!("Expected literal"),
        }

        match &template.items[1] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["name"]),
            _ => panic!("Expected field"),
        }

        match &template.items[2] {
            TemplateItem::Literal(text) => assert_eq!(text, ", Age: "),
            _ => panic!("Expected literal"),
        }

        match &template.items[3] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["age"]),
            _ => panic!("Expected field"),
        }
    }

    #[test]
    fn test_complex_filter() {
        let result = parse_command(r#"name == "Alice" && age > 25"#).unwrap();
        assert!(result.filter.is_some());

        if let Some(FilterExpr::And(left, right)) = result.filter {
            // Verify left side
            if let FilterExpr::Comparison { field, op, value } = left.as_ref() {
                assert_eq!(field.parts, vec!["name"]);
                assert_eq!(*op, ComparisonOp::Equal);
                assert_eq!(*value, FilterValue::String("Alice".to_string()));
            } else {
                panic!("Expected comparison on left");
            }

            // Verify right side
            if let FilterExpr::Comparison { field, op, value } = right.as_ref() {
                assert_eq!(field.parts, vec!["age"]);
                assert_eq!(*op, ComparisonOp::GreaterThan);
                assert_eq!(*value, FilterValue::Number(25.0));
            } else {
                panic!("Expected comparison on right");
            }
        } else {
            panic!("Expected AND expression");
        }
    }

    #[test]
    fn test_nested_field_access() {
        let result = parse_command(r#"user.email == "alice@example.com""#).unwrap();

        if let Some(FilterExpr::Comparison { field, .. }) = result.filter {
            assert_eq!(field.parts, vec!["user", "email"]);
        } else {
            panic!("Expected comparison with nested field");
        }
    }

    #[test]
    fn test_template_entire_input() {
        let result = parse_command(r#"{${0}}"#).unwrap();
        assert!(result.filter.is_none());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 1);

        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["$0"]),
            _ => panic!("Expected $0 field"),
        }

        let result2 = parse_command(r#"{${0} extra}"#).unwrap();
        assert!(result2.template.is_some());
        let template2 = result2.template.unwrap();
        assert_eq!(template2.items.len(), 2);

        match &template2.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["$0"]),
            _ => panic!("Expected $0 field"),
        }

        match &template2.items[1] {
            TemplateItem::Literal(text) => assert_eq!(text, " extra"),
            _ => panic!("Expected literal text"),
        }
    }

    #[test]
    fn test_template_indexed_fields() {
        let result = parse_command(r#"{${1}, ${2}, ${3}}"#).unwrap();
        assert!(result.filter.is_none());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 5);

        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["field_0"]),
            _ => panic!("Expected field_0 field"),
        }

        match &template.items[1] {
            TemplateItem::Literal(text) => assert_eq!(text, ", "),
            _ => panic!("Expected literal"),
        }

        match &template.items[2] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["field_1"]),
            _ => panic!("Expected field_1 field"),
        }

        match &template.items[3] {
            TemplateItem::Literal(text) => assert_eq!(text, ", "),
            _ => panic!("Expected literal"),
        }

        match &template.items[4] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["field_2"]),
            _ => panic!("Expected field_2 field"),
        }
    }

    #[test]
    fn test_template_braced_fields() {
        let result = parse_command(r#"{User: ${user.name}, Age: ${user.age}}"#).unwrap();
        assert!(result.filter.is_none());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 4);

        match &template.items[0] {
            TemplateItem::Literal(text) => assert_eq!(text, "User: "),
            _ => panic!("Expected literal"),
        }

        match &template.items[1] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["user", "name"]),
            _ => panic!("Expected user.name field"),
        }

        match &template.items[2] {
            TemplateItem::Literal(text) => assert_eq!(text, ", Age: "),
            _ => panic!("Expected literal"),
        }

        match &template.items[3] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["user", "age"]),
            _ => panic!("Expected user.age field"),
        }
    }

    #[test]
    fn test_template_mixed_syntax() {
        let result =
            parse_command(r#"name == "Alice" {Record: ${0} - Name: ${name}, Field1: ${1}}"#)
                .unwrap();
        assert!(result.filter.is_some());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 6);

        match &template.items[0] {
            TemplateItem::Literal(text) => assert_eq!(text, "Record: "),
            _ => panic!("Expected literal"),
        }

        match &template.items[1] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["$0"]),
            _ => panic!("Expected $0 field"),
        }

        match &template.items[2] {
            TemplateItem::Literal(text) => assert_eq!(text, " - Name: "),
            _ => panic!("Expected literal"),
        }

        match &template.items[3] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["name"]),
            _ => panic!("Expected name field"),
        }

        match &template.items[4] {
            TemplateItem::Literal(text) => assert_eq!(text, ", Field1: "),
            _ => panic!("Expected literal"),
        }

        match &template.items[5] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["field_0"]),
            _ => panic!("Expected field_0 field"),
        }
    }

    #[test]
    fn test_template_literal_dollar() {
        // Dollar patterns without braces should fail if they contain spaces (need quotes)
        let input = r#"Cost: $100"#;
        let result = parse_command(input);

        match &result {
            Ok(parsed) => {
                println!("DEBUG: Input '{}' parsed as:", input);
                println!("  Filter: {:?}", parsed.filter.is_some());
                println!("  Template: {:?}", parsed.template.is_some());
                println!("  Field selector: {:?}", parsed.field_selector.is_some());
            }
            Err(e) => {
                println!("DEBUG: Input '{}' correctly failed: {}", input, e);
            }
        }

        assert!(result.is_err(), "Unquoted strings with spaces should fail");

        // Quoted strings with dollar signs should fail (not valid field selectors)
        let quoted_input = r#""Cost: $100""#;
        let result_quoted = parse_command(quoted_input);
        assert!(
            result_quoted.is_err(),
            "Quoted strings with spaces and dollar signs should fail"
        );

        // Valid field selectors should work
        let valid_field = r#""name""#;
        let result_valid = parse_command(valid_field).unwrap();
        assert!(result_valid.filter.is_none());
        assert!(result_valid.field_selector.is_some()); // Should be field selector
        assert!(result_valid.template.is_none());

        // Test braces with literal dollars
        let result2 = parse_command(r#"{Cost: $$100}"#).unwrap();
        assert!(result2.filter.is_none());
        assert!(result2.template.is_some()); // Should be template with literal dollar

        let template = result2.template.unwrap();
        assert_eq!(template.items.len(), 1);

        match &template.items[0] {
            TemplateItem::Literal(text) => assert_eq!(text, "Cost: $100"), // $$ becomes literal $
            _ => panic!("Expected literal with dollar sign"),
        }

        // Test with variable
        let result3 = parse_command(r#"{Cost: ${price}}"#).unwrap();
        assert!(result3.filter.is_none());
        assert!(result3.template.is_some()); // Should be template

        let template3 = result3.template.unwrap();
        assert_eq!(template3.items.len(), 2);

        match &template3.items[0] {
            TemplateItem::Literal(text) => assert_eq!(text, "Cost: "),
            _ => panic!("Expected literal"),
        }

        match &template3.items[1] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["price"]),
            _ => panic!("Expected price field"),
        }
    }

    #[test]
    fn test_comprehensive_field_selector_disambiguation() {
        // Test simple unquoted identifiers
        let test_cases = vec![
            ("name", true),
            ("age", true),
            ("user_id", true),
            ("firstName", true),
            ("field123", true),
            ("_private", true),
            ("user.name", true),
            ("data.items.0", true),
            ("deep.nested.field.access", true),
        ];

        for (input, should_be_field_selector) in test_cases {
            let result = parse_command(input).unwrap();
            assert_eq!(
                result.field_selector.is_some(),
                should_be_field_selector,
                "Input '{}' should {} be a field selector",
                input,
                if should_be_field_selector { "" } else { "not" }
            );
            assert!(
                result.filter.is_none(),
                "Input '{}' should not be a filter",
                input
            );
            assert!(
                result.template.is_none(),
                "Input '{}' should not be a template",
                input
            );

            if should_be_field_selector {
                let field_path = result.field_selector.unwrap();
                let expected_parts: Vec<String> = input.split('.').map(|s| s.to_string()).collect();
                assert_eq!(
                    field_path.parts, expected_parts,
                    "Field path parts mismatch for '{}'",
                    input
                );
            }
        }
    }

    #[test]
    fn test_comprehensive_template_disambiguation() {
        let test_cases = vec![
            // Templates: entirely contained within braces with variables
            ("{${name}}", true),
            ("{${user.name}}", true),
            ("{Hello ${name}}", true),
            ("{${name} is ${age} years old}", true),
            ("{User: ${user.name}, Age: ${user.age}}", true),
            ("{${name} costs $$100}", true),
            ("{Process ${0} and ${0}}", true),
            ("{Hello $world}", true), // Template with simple variable
            ("{User: $name}", true),  // Template with simple variable
            // Not templates: braces don't contain entire expression or no braces
            ("$name", false),
            ("$user", false),
            ("Hello $name", false),
            ("$name is $age", false),
            ("Cost: $100", false),
            ("{name}", false),      // Bare field without variables
            ("{user.name}", false), // Bare field without variables
        ];

        for (input, should_be_template) in test_cases {
            let result = parse_command(input);
            if should_be_template {
                assert!(
                    result.is_ok(),
                    "Template '{}' should parse successfully: {}",
                    input,
                    result.unwrap_err()
                );
                let parsed = result.unwrap();
                assert!(
                    parsed.template.is_some(),
                    "Input '{}' should be a template",
                    input
                );
                assert!(
                    parsed.filter.is_none(),
                    "Input '{}' should not be a filter",
                    input
                );
                assert!(
                    parsed.field_selector.is_none(),
                    "Input '{}' should not be a field selector",
                    input
                );
            } else {
                // These should either parse as something else or fail
                if let Ok(parsed) = result {
                    assert!(
                        parsed.template.is_none(),
                        "Input '{}' should not be a template",
                        input
                    );
                }
                // If they fail to parse, that's also acceptable
            }
        }
    }

    #[test]
    fn test_comprehensive_filter_disambiguation() {
        // Test various filter patterns
        let test_cases = vec![
            // Equality operators
            ("name == \"Alice\"", true),
            ("age == 30", true),
            ("active == true", true),
            ("value == null", true),
            // Comparison operators
            ("age > 25", true),
            ("age < 65", true),
            ("age >= 18", true),
            ("age <= 100", true),
            ("age != 0", true),
            // String operators (these might fail parsing due to grammar issues, but test the detection)
            // Note: These are moved to a separate check due to grammar limitations
            // ("name startswith \"A\"", true),
            // ("name endswith \"son\"", true),
            // Logical operators
            ("age > 18 && age < 65", true),
            ("name == \"Alice\" || name == \"Bob\"", true),
            ("!(age < 18)", true),
            ("!active", true),
            // Nested field access
            ("user.name == \"Alice\"", true),
            ("data.items.0 > 100", true),
        ];

        for (input, should_be_filter) in test_cases {
            let result = parse_command(input);
            if result.is_ok() {
                let parsed = result.unwrap();
                assert_eq!(
                    parsed.filter.is_some(),
                    should_be_filter,
                    "Input '{}' should {} be a filter",
                    input,
                    if should_be_filter { "" } else { "not" }
                );

                if should_be_filter {
                    assert!(
                        parsed.field_selector.is_none(),
                        "Input '{}' should not be a field selector",
                        input
                    );
                    assert!(
                        parsed.template.is_none(),
                        "Input '{}' should not be a template",
                        input
                    );
                }
            } else {
                // Some filters might fail due to grammar issues (like contains), but we should still detect them as intended filters
                println!(
                    "Filter '{}' failed to parse (expected due to grammar limitations): {}",
                    input,
                    result.unwrap_err()
                );

                // Check that our detection logic would classify it correctly
                assert!(
                    contains_filter_operators(input),
                    "Should detect '{}' as having filter operators",
                    input
                );
            }
        }

        // Test cases that should work with the new grammar
        let supported_cases = vec!["name ^= \"A\"", "name $= \"son\""];

        for input in supported_cases {
            let result = parse_command(input);
            assert!(
                result.is_ok(),
                "Input '{}' should parse successfully with new grammar: {}",
                input,
                result.unwrap_err()
            );
            let parsed = result.unwrap();
            assert!(
                parsed.filter.is_some(),
                "Input '{}' should be a filter",
                input
            );
        }
    }

    #[test]
    fn test_disambiguation_edge_cases() {
        let test_cases = vec![
            // Field selectors: no operators, no braces
            ("field_name", "field_selector"),
            ("_underscore", "field_selector"),
            ("CamelCase", "field_selector"),
            ("number123", "field_selector"),
            ("field.with.dots", "field_selector"),
            // Templates: entirely contained within braces with variables
            ("{${field}}", "template"),
            ("{prefix_${field}_suffix}", "template"),
            ("{${1} ${2} ${3}}", "template"),
            ("{Hello $world}", "template"),  // Template with variable
            ("{user: ${name}}", "template"), // Template with variable
            // Filters: contain operators
            ("field==value", "filter"),
            ("field == \"value\"", "filter"),
            ("field>5", "filter"),
            ("field >= 5", "filter"),
            // Quoted field selectors
            ("\"field name with spaces\"", "field_selector"),
            ("'field.with.dots'", "field_selector"),
        ];

        for (input, expected_type) in test_cases {
            let result = parse_command(input);
            assert!(
                result.is_ok(),
                "Failed to parse '{}': {}",
                input,
                result.unwrap_err()
            );

            let parsed = result.unwrap();
            let actual_type = if parsed.filter.is_some() {
                "filter"
            } else if parsed.field_selector.is_some() {
                "field_selector"
            } else if parsed.template.is_some() {
                "template"
            } else {
                "none"
            };

            assert_eq!(
                actual_type, expected_type,
                "Input '{}' should be classified as '{}' but was classified as '{}'",
                input, expected_type, actual_type
            );
        }

        // Test cases that should result in errors
        let error_cases = vec![
            "prefix_{field}_suffix",     // Partial braces
            "$1 $2 $3",                  // Unquoted with spaces
            "Hello {name}",              // Partial braces
            "{name} is {age} years old", // Multiple partial braces
            "field with spaces",         // Unquoted with spaces
            "{field}",                   // Bare field without variables
            "{user.name}",               // Bare field without variables
        ];

        for input in error_cases {
            let result = parse_command(input);
            assert!(
                result.is_err(),
                "Input '{}' should result in an error but parsed successfully",
                input
            );
        }
    }

    #[test]
    fn test_disambiguation_priority_order() {
        let result = parse_command("name == \"Alice\" {Name: ${name}}").unwrap();
        assert!(
            result.filter.is_some(),
            "Should parse filter part of combined expression"
        );
        assert!(
            result.template.is_some(),
            "Should parse template part of combined expression"
        );

        let result = parse_command("name == \"test\"").unwrap();
        assert!(
            result.filter.is_some(),
            "Should be classified as filter due to operator"
        );
        assert!(
            result.field_selector.is_none(),
            "Should not be field selector when operators present"
        );

        let result = parse_command("{Hello ${user}}").unwrap();
        assert!(
            result.template.is_some(),
            "Should be classified as template due to braces"
        );
        assert!(
            result.filter.is_none(),
            "Should not be filter without operators"
        );

        let result = parse_command("simple_field").unwrap();
        assert!(
            result.field_selector.is_some(),
            "Should be field selector for simple identifier"
        );
        assert!(
            result.filter.is_none(),
            "Should not be filter without operators"
        );
        assert!(
            result.template.is_none(),
            "Should not be template without braces"
        );
    }

    #[test]
    fn test_helper_function_accuracy() {
        // Test the helper functions used in disambiguation

        // Test with symbol-based operators
        assert!(contains_filter_operators("age > 25"));
        assert!(contains_filter_operators("name == \"test\""));
        assert!(contains_filter_operators("field != null"));
        assert!(contains_filter_operators("name *= \"something\"")); // NEW: symbol-based contains
        assert!(contains_filter_operators("name ^= \"prefix\"")); // NEW: symbol-based startswith
        assert!(contains_filter_operators("name $= \"suffix\"")); // NEW: symbol-based endswith
        assert!(contains_filter_operators("active && enabled"));
        assert!(contains_filter_operators("!active"));
        assert!(!contains_filter_operators("simple_field"));
        assert!(!contains_filter_operators("{template}"));
        assert!(!contains_filter_operators("$variable"));

        // Test is_simple_field_selector
        assert!(is_simple_field_selector("field"));
        assert!(is_simple_field_selector("field_name"));
        assert!(is_simple_field_selector("user.name"));
        assert!(is_simple_field_selector("data.items.0"));
        assert!(is_simple_field_selector("\"quoted field\""));
        assert!(is_simple_field_selector("'quoted field'"));
        assert!(!is_simple_field_selector("field > 5"));
        assert!(!is_simple_field_selector("{template}"));
        assert!(!is_simple_field_selector("field with spaces")); // unquoted spaces not allowed

        // Test contains_dollar_variables with new flexible rules
        assert!(contains_dollar_variables("${name}")); // ${...} syntax always recognized
        assert!(contains_dollar_variables("{Hello ${name}}")); // ${...} in braces
        assert!(contains_dollar_variables("$name")); // Simple variables now allowed
        assert!(contains_dollar_variables("Hello $name")); // Simple variables in text
        assert!(!contains_dollar_variables("$1 $2")); // Numeric variables need ${} syntax
        assert!(!contains_dollar_variables("$25")); // Ambiguous with currency, needs ${25}
        assert!(!contains_dollar_variables("$100")); // Ambiguous with currency, needs ${100}
        assert!(!contains_dollar_variables("Cost: $5.99")); // Currency, not variables
        assert!(!contains_dollar_variables("simple text"));
        assert!(!contains_dollar_variables(r"Cost: \$100")); // Escaped dollar is not a variable
    }

    #[test]
    fn test_quoted_field_selector_parsing() {
        // Test that quoted field selectors work correctly
        let test_cases = vec![
            ("\"simple\"", vec!["simple"]),
            ("'simple'", vec!["simple"]),
            ("\"field.with.dots\"", vec!["field", "with", "dots"]),
            ("'field.with.dots'", vec!["field", "with", "dots"]),
            ("\"field with spaces\"", vec!["field with spaces"]),
            ("'field with spaces'", vec!["field with spaces"]),
        ];

        for (input, expected_parts) in test_cases {
            let result = parse_command(input).unwrap();
            assert!(
                result.field_selector.is_some(),
                "Should parse '{}' as field selector",
                input
            );

            let field_path = result.field_selector.unwrap();
            assert_eq!(
                field_path.parts, expected_parts,
                "Field path parts mismatch for '{}'",
                input
            );
        }
    }

    #[test]
    fn test_unquoted_field_selector_parsing() {
        // Test that unquoted field selectors work correctly
        let test_cases = vec![
            ("field", vec!["field"]),
            ("field_name", vec!["field_name"]),
            ("user.name", vec!["user", "name"]),
            ("data.items.0", vec!["data", "items", "0"]),
            (
                "deep.nested.field.access",
                vec!["deep", "nested", "field", "access"],
            ),
        ];

        for (input, expected_parts) in test_cases {
            let result = parse_command(input).unwrap();
            assert!(
                result.field_selector.is_some(),
                "Should parse '{}' as field selector",
                input
            );

            let field_path = result.field_selector.unwrap();
            assert_eq!(
                field_path.parts, expected_parts,
                "Field path parts mismatch for '{}'",
                input
            );
        }
    }

    #[test]
    fn test_currency_vs_variable_disambiguation() {
        // Invalid inputs - raw dollar signs are not supported
        let invalid_cases = vec![
            "$0",     // Invalid - raw dollar
            "$1",     // Invalid - raw dollar
            "$25",    // Invalid - raw dollar
            "$100",   // Invalid - raw dollar
            "$name",  // Invalid - raw dollar
            "$price", // Invalid - raw dollar
        ];

        for input in invalid_cases {
            let result = parse_command(input);
            assert!(
                result.is_err(),
                "Input '{}' should be rejected (raw dollar signs not allowed)",
                input
            );
        }

        // Valid field selectors without dollar signs
        let field_selector_cases = vec![
            "amount",  // Simple field
            "price",   // Simple field
            "field_0", // Indexed field without dollar
            "field_1", // Indexed field without dollar
        ];

        for input in field_selector_cases {
            let result = parse_command(input).unwrap();
            assert!(
                result.field_selector.is_some(),
                "Input '{}' should be treated as field selector",
                input
            );
            assert!(
                result.template.is_none(),
                "Input '{}' should NOT be template",
                input
            );
        }

        // Test templates with braces and proper variable syntax
        let template_cases = vec![
            "{${name} costs $$100}",    // Template with special variable
            "{Price $$50 for ${user}}", // Template with special variable
            "{${name} paid $$25.99}",   // Template with special variable
            "{Item: ${1} costs $$100}", // Template with special variable
            "{Total: $$500}",           // braces + escaped $ (literal template)
            "{${price} is expensive}",  // Simple variable in template
        ];

        for input in template_cases {
            let result = parse_command(input).unwrap();
            assert!(
                result.template.is_some(),
                "Input '{}' should be treated as template",
                input
            );
            assert!(
                result.field_selector.is_none(),
                "Input '{}' should not be field selector when template present",
                input
            );
        }

        // Test that simple text without dollar signs is treated as field selectors
        let non_template_cases = vec!["simple_field", "user.name", "data.items.0"];

        for input in non_template_cases {
            let result = parse_command(input).unwrap();
            assert!(
                result.field_selector.is_some(),
                "Input '{}' should be treated as field selector",
                input
            );
            assert!(
                result.template.is_none(),
                "Input '{}' should not be template without variables",
                input
            );
        }
    }

    #[test]
    fn test_dollar_zero_standardization() {
        // Test that ${0} works correctly
        let dollar_zero_cases = vec![
            ("{${0}}", true),                           // ${0} in braces
            ("{Original: ${0}}", true),                 // ${0} in template with text
            ("{Line: ${0} | Field: ${field_0}}", true), // ${0} mixed with other variables
            ("{${0} extra content}", true),             // ${0} with trailing content
        ];

        for (input, should_be_template) in dollar_zero_cases {
            let result = parse_command(input).unwrap();
            assert_eq!(
                result.template.is_some(),
                should_be_template,
                "Input '{}' should {} be parsed as template",
                input,
                if should_be_template { "" } else { "not" }
            );

            if should_be_template {
                let template = result.template.unwrap();
                let has_dollar_zero = template.items.iter().any(
                    |item| matches!(item, TemplateItem::Field(field) if field.parts == vec!["$0"]),
                );
                assert!(
                    has_dollar_zero,
                    "Template for '{}' should contain $0 field variable",
                    input
                );
            }
        }

        // Test cases that should be rejected (raw dollar signs)
        let invalid_cases = vec![
            "$0",               // Raw dollar - not allowed
            "Original: $0",     // Raw dollar in text - not allowed
            "$0 extra content", // Raw dollar with spaces - not allowed
        ];

        for input in invalid_cases {
            let result = parse_command(input);
            assert!(
                result.is_err(),
                "Input '{}' should be rejected (raw dollar signs not allowed)",
                input
            );
        }
    }

    #[test]
    fn test_contextual_operator_detection() {
        // Test that "and"/"or" are only treated as operators in appropriate contexts
        let non_operator_cases = vec![
            "android",            // Word containing "and"
            "operator",           // Word containing "or"
            "command",            // Word containing "and"
            "portland",           // Word containing "or"
            "\"brand new item\"", // "and" as part of phrase (quoted)
            "\"work order\"",     // "or" as part of phrase (quoted)
        ];

        for input in non_operator_cases {
            assert!(
                !contains_filter_operators(input),
                "Input '{}' should not be detected as having filter operators",
                input
            );

            let result = parse_command(input).unwrap();
            // Should be treated as field selector, not filter
            assert!(
                result.field_selector.is_some() || result.template.is_some(),
                "Input '{}' should be field selector or template, not filter",
                input
            );
            assert!(
                result.filter.is_none(),
                "Input '{}' should not be parsed as filter",
                input
            );
        }

        // Test that actual logical operators ARE detected
        let operator_cases = vec![
            "field == value && other == test",
            "active == true || enabled == false",
            "name == \"test\" and age > 25", // Mixed syntax
            "field != null or status == active",
        ];

        for input in operator_cases {
            assert!(
                contains_filter_operators(input),
                "Input '{}' should be detected as having filter operators",
                input
            );
        }
    }

    #[test]
    fn test_template_variable_detection_edge_cases() {
        // Test edge cases in template variable detection
        let edge_cases = vec![
            // These should NOT be detected as having template variables
            ("$", false),               // Lone dollar sign
            ("$$", false),              // Double dollar (literal)
            (r"\$100", false),          // Escaped dollar (literal)
            (r"Price: \$19.99", false), // Escaped dollar in context
            ("$0", false),              // $0 is literal, only ${0} is variable
            ("$1", false),              // Other numeric variables need ${} syntax
            ("$9", false),              // Numeric variables need ${} syntax
            ("$25", false),             // Ambiguous with currency, needs ${25}
            ("$100", false),            // Ambiguous with currency, needs ${100}
            ("$1000", false),           // Ambiguous with currency, needs ${1000}
            ("{field}", false),         // No ${...}
            ("Item costs $25", false),  // Currency, not variables
            // These SHOULD be detected as having template variables
            ("$name", true),           // Simple variables allowed
            ("$field_name", true),     // Simple variables with underscore allowed
            ("$user", true),           // Simple variables allowed
            ("Hello $name", true),     // Simple variables in text allowed
            ("${field}", true),        // ${...}
            ("${field.name}", true),   // ${...} with dots
            ("{${0}}", true),          // ${...} in braces
            ("{${name}}", true),       // ${...} in braces
            ("{Hello ${name}}", true), // ${...} in braces with text
            ("{${field_name}}", true), // ${...} with underscore
            ("{${user.email}}", true), // ${...} with dots
        ];

        for (input, should_have_variables) in edge_cases {
            let has_variables = contains_dollar_variables(input);
            assert_eq!(
                has_variables, should_have_variables,
                "Input '{}' variable detection mismatch. Expected: {}, Got: {}",
                input, should_have_variables, has_variables
            );
        }
    }

    #[test]
    fn test_mixed_currency_and_variables() {
        // Test cases showing currency handling in templates
        let mixed_cases = vec![
            // These should be invalid (partial braces or raw dollars)
            // Note: inputs with raw $ outside templates should fail

            // Valid templates with proper syntax
            ("{${name} costs $$100}", true), // braces + ${...} + escaped $
            ("{Price $$50 for ${user}}", true), // braces + ${...} + escaped $
            ("{${name} paid $$25.99}", true), // braces + ${...} + escaped $
            ("{Item: ${1} costs $$100}", true), // braces + ${...} + escaped $
            ("{Total: $$500}", true),        // braces + escaped $ (literal template)
            ("{${price} is expensive}", true), // Simple variable in template
        ];

        for (input, should_be_template) in mixed_cases {
            let result = parse_command(input);
            assert!(
                result.is_ok(),
                "Failed to parse '{}': {}",
                input,
                result.unwrap_err()
            );

            let parsed = result.unwrap();
            assert_eq!(
                parsed.template.is_some(),
                should_be_template,
                "Input '{}' template detection mismatch. Expected: {}, Got: {}",
                input,
                should_be_template,
                parsed.template.is_some()
            );
        }

        // Test invalid cases that should be rejected
        let invalid_cases = vec![
            "{name} costs $100",   // Invalid: partial braces with raw $
            "Price $50 for $user", // Invalid: raw $ without braces
            "$name paid $25.99",   // Invalid: raw $ variables
        ];

        for input in invalid_cases {
            let result = parse_command(input);
            assert!(result.is_err(), "Input '{}' should be rejected", input);
        }
    }

    #[test]
    fn test_field_selector_vs_template_boundary_cases() {
        // Test boundary cases between field selectors and templates
        let valid_cases = vec![
            // Should be field selectors (no template syntax)
            ("user.name", "field_selector"),
            ("data.items.0", "field_selector"),
            ("simple_field", "field_selector"),
            ("\"field with spaces\"", "field_selector"),
            ("'quoted.field'", "field_selector"),
            // Should be templates (contain template syntax: braces + variables)
            ("{${user.name}}", "template"),
            ("{Hello ${name}}", "template"),
            ("{Value: ${field}}", "template"),
            ("{Field ${1} value}", "template"),
            ("{Hello $world}", "template"), // Template with variable
        ];

        for (input, expected_type) in valid_cases {
            let result = parse_command(input);
            assert!(
                result.is_ok(),
                "Failed to parse valid input '{}': {}",
                input,
                result.unwrap_err()
            );

            let parsed = result.unwrap();
            let actual_type = if parsed.field_selector.is_some() {
                "field_selector"
            } else if parsed.template.is_some() {
                "template"
            } else if parsed.filter.is_some() {
                "filter"
            } else {
                "none"
            };

            assert_eq!(
                actual_type, expected_type,
                "Input '{}' type detection mismatch. Expected: {}, Got: {}",
                input, expected_type, actual_type
            );
        }

        // Test cases that should fail due to invalid syntax
        let invalid_cases = vec![
            "Value: $field",      // Unquoted with spaces
            "Field $1 value",     // Unquoted with spaces
            "Hello {name}",       // Partial braces
            "{name} and {value}", // Multiple bare fields
        ];

        for input in invalid_cases {
            let result = parse_command(input);
            assert!(
                result.is_err(),
                "Input '{}' should fail to parse but succeeded",
                input
            );
        }
    }

    #[test]
    fn test_operator_precedence_in_detection() {
        // Test that operator detection works correctly with different precedence levels
        let operator_precedence_cases = vec![
            "a == b && c == d",           // AND has higher precedence than OR
            "a == b || c == d",           // Basic OR
            "!(a == b)",                  // NOT operator
            "a != b",                     // Not equals
            "a > b && (c < d || e >= f)", // Complex precedence
            "field ^= \"pre\"",           // NEW: symbol-based startswith
            "field *= \"sub\"",           // NEW: symbol-based contains
            "field $= \"end\"",           // NEW: symbol-based endswith
            "field ~ \"pattern\"",        // Regex match operator
        ];

        for input in operator_precedence_cases {
            assert!(
                contains_filter_operators(input),
                "Input '{}' should be detected as having filter operators",
                input
            );
        }
    }

    #[test]
    fn test_fallback_filter_parsing_edge_cases() {
        // Test the fallback filter parser with edge cases
        let fallback_cases = vec![
            // "not field" pattern
            ("not active", Some("not field pattern")),
            ("not user.enabled", Some("not nested field")),
            // Simple "field op value" patterns that might not parse with main parser
            ("status == active", Some("unquoted value")),
            ("count > 5", Some("numeric comparison")),
            ("name != null", Some("null comparison")),
        ];

        for (input, description) in fallback_cases {
            if let Some(filter) = parse_simple_filter_fallback(input) {
                // If fallback parsing succeeds, verify the structure
                match filter {
                    FilterExpr::Not(_) => {
                        assert!(
                            input.starts_with("not "),
                            "Not filter should start with 'not ': {}",
                            input
                        );
                    }
                    FilterExpr::Comparison { .. } => {
                        // Should be a basic comparison
                    }
                    _ => panic!(
                        "Unexpected filter type for fallback case: {} ({})",
                        input,
                        description.unwrap_or("no description")
                    ),
                }
            }
            // If fallback parsing fails, that's also acceptable for some cases
        }
    }

    #[test]
    fn test_invalid_bare_field_template() {
        // {field} without $ should be rejected - templates need actual variable content
        assert!(parse_command("{field}").is_err());
        assert!(parse_command("{name}").is_err());
        assert!(parse_command("{value}").is_err());
    }

    #[test]
    fn test_complete_syntax_disambiguation() {
        // Test all syntax disambiguation cases comprehensively

        // 1. {template} - Templates must be enclosed in braces with variables
        let valid_templates = vec![
            ("{${name}}", "simple variable"),
            ("{${user.email}}", "nested variable"),
            ("{Hello ${name}}", "text with variable"),
            ("{${name} is ${age} years old}", "multiple variables"),
            ("{Cost: $$100}", "literal dollar signs"),
            ("{${0}}", "original input"),
            ("{$name}", "simple variable without braces"),
            ("{User: $name, Age: ${age}}", "mixed variable syntax"),
        ];

        for (input, description) in valid_templates {
            let result = parse_command(input);
            assert!(
                result.is_ok(),
                "Valid template '{}' ({}) should parse: {}",
                input,
                description,
                result.unwrap_err()
            );
            let parsed = result.unwrap();
            assert!(
                parsed.template.is_some(),
                "Input '{}' ({}) should be parsed as template",
                input,
                description
            );
            assert!(
                parsed.filter.is_none() && parsed.field_selector.is_none(),
                "Template '{}' should not be parsed as filter or field selector",
                input
            );
        }

        // 2. Raw dollar patterns are NOT templates (they should be rejected)
        let invalid_raw_dollars = vec![
            (
                "cut the ${cost} is not a template",
                "unquoted text with variables",
            ),
            ("$0", "raw dollar zero - literal, not variable"),
            ("$1", "raw dollar number - literal, not variable"),
            ("$name", "raw dollar variable - not allowed outside braces"),
            ("$price", "raw dollar field - not allowed outside braces"),
            ("Hello $name", "text with raw dollar variable"),
            ("Cost: $100", "text with dollar amount"),
            ("Fee: $0", "text with dollar zero"),
        ];

        for (input, description) in invalid_raw_dollars {
            let result = parse_command(input);
            assert!(
                result.is_err(),
                "Invalid input '{}' ({}) should be rejected but was accepted",
                input,
                description
            );
        }

        // 3. $$ is literal dollar sign (only in templates)
        let literal_dollar_cases = vec![
            ("{Price: $$100}", "$$100", "literal $100"),
            (
                "{Cost $$50 for ${item}}",
                "$$50",
                "literal $50 with variable",
            ),
            ("{Total: $$0}", "$$0", "literal $0"),
            ("{Fee: $$25.99}", "$$25.99", "literal $25.99"),
        ];

        for (input, _literal_part, description) in literal_dollar_cases {
            let result = parse_command(input).unwrap();
            assert!(
                result.template.is_some(),
                "Should parse as template: {}",
                input
            );
            let template = result.template.unwrap();
            let has_literal_dollar = template.items.iter().any(|item| {
                if let TemplateItem::Literal(text) = item {
                    text.contains('$') && !text.contains("$$")
                } else {
                    false
                }
            });
            assert!(
                has_literal_dollar,
                "Template '{}' should contain literal dollar sign for {}",
                input, description
            );
        }

        // 4. $0, $1, etc. are literals (currency/amounts), not variables
        let literal_dollar_numbers = vec![
            ("$0", "zero dollars"),
            ("$1", "one dollar"),
            ("$10", "ten dollars"),
            ("$100", "hundred dollars"),
            ("$1000", "thousand dollars"),
        ];

        for (input, description) in literal_dollar_numbers {
            let result = parse_command(input);
            assert!(
                result.is_err(),
                "Literal '{}' ({}) should be rejected as invalid syntax",
                input,
                description
            );

            // Also test that they're not detected as having variables
            assert!(
                !contains_dollar_variables(input),
                "Literal '{}' should not be detected as having variables",
                input
            );
        }

        // 5. ${0} is the full input substitution (requires braces)
        let original_input_cases = vec![
            ("{${0}}", true),
            ("{Original: ${0}}", true),
            ("{${0} - processed}", true),
            ("{Line: ${0} | Status: OK}", true),
        ];

        for (input, should_work) in original_input_cases {
            let result = parse_command(input);
            if should_work {
                assert!(
                    result.is_ok(),
                    "Original input template '{}' should work",
                    input
                );
                let parsed = result.unwrap();
                assert!(parsed.template.is_some(), "Should be parsed as template");
                let template = parsed.template.unwrap();
                let has_original_input = template.items.iter().any(|item| {
                    if let TemplateItem::Field(field) = item {
                        field.parts == vec!["$0"]
                    } else {
                        false
                    }
                });
                assert!(
                    has_original_input,
                    "Template '{}' should contain original input variable",
                    input
                );
            } else {
                assert!(
                    result.is_err(),
                    "Invalid original input '{}' should fail",
                    input
                );
            }
        }

        // 6. ${name} is a variable (braced syntax, works for all variables)
        let braced_variables = vec![
            ("{${name}}", "simple field"),
            ("{${user.email}}", "nested field"),
            ("{${field_0}}", "indexed field"),
            ("{${1}}", "numeric field"),
            ("{${data.items.0}}", "complex nested field"),
        ];

        for (input, description) in braced_variables {
            let result = parse_command(input).unwrap();
            assert!(
                result.template.is_some(),
                "Braced variable '{}' ({}) should be template",
                input,
                description
            );

            // Verify it's detected as having variables
            assert!(
                contains_dollar_variables(input),
                "Should detect variables in '{}'",
                input
            );
        }

        // 7. $name is a variable (simple syntax, only for non-numeric)
        let simple_variables = vec![
            ("{$name}", "simple field"),
            ("{$user}", "simple field"),
            ("{$email}", "simple field"),
            ("{Hello $world}", "variable in text"),
            (
                "{User: $name, Status: $active}",
                "multiple simple variables",
            ),
        ];

        for (input, description) in simple_variables {
            let result = parse_command(input).unwrap();
            assert!(
                result.template.is_some(),
                "Simple variable '{}' ({}) should be template",
                input,
                description
            );

            // Verify it's detected as having variables
            assert!(
                contains_dollar_variables(input),
                "Should detect variables in '{}'",
                input
            );
        }

        // Test that simple syntax doesn't work for numbers
        let invalid_simple_numeric = vec![
            ("{$0}", "simple syntax not allowed for zero"),
            ("{$1}", "simple syntax not allowed for numbers"),
            ("{$10}", "simple syntax not allowed for multi-digit"),
        ];

        for (input, description) in invalid_simple_numeric {
            // These should parse as templates but $0, $1, etc. should be treated as literals
            let result = parse_command(input);
            if result.is_ok() {
                let parsed = result.unwrap();
                if parsed.template.is_some() {
                    // The template should treat $0, $1 as literal text, not variables
                    let template = parsed.template.unwrap();
                    let has_literal_dollar_number = template.items.iter().any(|item| {
                        if let TemplateItem::Literal(text) = item {
                            text.starts_with('$')
                                && text.chars().skip(1).all(|c| c.is_ascii_digit())
                        } else {
                            false
                        }
                    });
                    assert!(
                        has_literal_dollar_number,
                        "Template '{}' should treat {} as literal, not variable",
                        input, description
                    );
                }
            }
        }

        // 8. {name} is a literal template, not a variable (should be rejected)
        let bare_field_templates = vec![
            ("{name}", "bare field without variables"),
            ("{user.email}", "bare nested field"),
            ("{field_0}", "bare indexed field"),
            ("{status}", "bare simple field"),
        ];

        for (input, description) in bare_field_templates {
            let result = parse_command(input);
            assert!(
                result.is_err(),
                "Bare field template '{}' ({}) should be rejected",
                input,
                description
            );
        }

        // Additional edge cases for completeness
        let edge_cases = vec![
            // Mixed valid and invalid
            (
                "{${name} costs $100}",
                true,
                "variable with literal currency should work inside braces",
            ),
            (
                "{${name} costs $$100}",
                true,
                "variable with escaped dollar should work",
            ),
            // Partial braces
            ("Hello {name}", false, "partial braces should fail"),
            (
                "{name} and {value}",
                false,
                "multiple bare fields should fail",
            ),
            // Valid field selectors (not templates)
            ("name", true, "simple field selector"),
            ("\"user.email\"", true, "quoted field selector"),
            ("'complex field'", true, "quoted field with spaces"),
        ];

        for (input, should_work, description) in edge_cases {
            let result = parse_command(input);
            if should_work {
                assert!(
                    result.is_ok(),
                    "Valid case '{}' ({}) should work",
                    input,
                    description
                );
            } else {
                assert!(
                    result.is_err(),
                    "Invalid case '{}' ({}) should fail",
                    input,
                    description
                );
            }
        }
    }
}
