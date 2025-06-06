// dsl_parser.rs - Converts Pest parse tree to AST

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::filter::{ComparisonOp, FieldPath, FilterExpr, FilterValue, Template, TemplateItem};

#[derive(Parser)]
#[grammar = "pest/parsm.pest"]
pub struct DSLParser;

#[derive(Debug)]
pub struct ParsedDSL {
    pub filter: Option<FilterExpr>,
    pub template: Option<Template>,
}

impl DSLParser {
    pub fn parse_dsl(input: &str) -> Result<ParsedDSL, pest::error::Error<Rule>> {
        let mut pairs = Self::parse(Rule::program, input)?;
        let program = pairs.next().unwrap();

        let mut result = ParsedDSL {
            filter: None,
            template: None,
        };

        for pair in program.into_inner() {
            match pair.as_rule() {
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

    fn parse_filter_expr(pair: Pair<Rule>) -> Result<FilterExpr, pest::error::Error<Rule>> {
        let inner = pair.into_inner().next().unwrap();
        Self::parse_condition(inner)
    }

    fn parse_condition(pair: Pair<Rule>) -> Result<FilterExpr, pest::error::Error<Rule>> {
        let inner = pair.into_inner().next().unwrap();
        Self::parse_or_expr(inner)
    }

    fn parse_or_expr(pair: Pair<Rule>) -> Result<FilterExpr, pest::error::Error<Rule>> {
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

    fn parse_and_expr(pair: Pair<Rule>) -> Result<FilterExpr, pest::error::Error<Rule>> {
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

    fn parse_comparison(pair: Pair<Rule>) -> Result<FilterExpr, pest::error::Error<Rule>> {
        let mut inner = pair.into_inner();
        let first = inner.next().unwrap();

        match first.as_rule() {
            Rule::not_op => {
                let comparison = Self::parse_comparison(inner.next().unwrap())?;
                Ok(FilterExpr::Not(Box::new(comparison)))
            }
            Rule::field_access => {
                let field = Self::parse_field_access(first);
                let op_pair = inner.next().unwrap();
                let op = Self::parse_comparison_op(op_pair);
                let value_pair = inner.next().unwrap();
                let value = Self::parse_value(value_pair);

                Ok(FilterExpr::Comparison { field, op, value })
            }
            _ => {
                // Handle parenthesized expressions or other cases
                Self::parse_condition(first)
            }
        }
    }

    fn parse_field_access(pair: Pair<Rule>) -> FieldPath {
        let parts: Vec<String> = pair
            .into_inner()
            .map(|identifier| identifier.as_str().to_string())
            .collect();
        FieldPath::new(parts)
    }

    fn parse_comparison_op(pair: Pair<Rule>) -> ComparisonOp {
        match pair.as_str() {
            "==" => ComparisonOp::Equal,
            "!=" => ComparisonOp::NotEqual,
            "<" => ComparisonOp::LessThan,
            "<=" => ComparisonOp::LessThanOrEqual,
            ">" => ComparisonOp::GreaterThan,
            ">=" => ComparisonOp::GreaterThanOrEqual,
            "contains" => ComparisonOp::Contains,
            "startswith" => ComparisonOp::StartsWith,
            "endswith" => ComparisonOp::EndsWith,
            "matches" => ComparisonOp::Matches,
            _ => ComparisonOp::Equal, // Default fallback
        }
    }

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
            _ => FilterValue::String(inner.as_str().to_string()),
        }
    }

    fn parse_template_expr(pair: Pair<Rule>) -> Result<Template, pest::error::Error<Rule>> {
        let template_str = pair.as_str();
        let items = Self::parse_template_string(template_str);
        Ok(Template { items })
    }

    fn parse_template_string(template_str: &str) -> Vec<TemplateItem> {
        let mut items = Vec::new();
        let mut chars = template_str.chars().peekable();
        let mut current_text = String::new();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                // Check what follows the $
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '$' {
                        // $$ = entire input
                        // If we have accumulated text, add it as a literal first
                        if !current_text.is_empty() {
                            items.push(TemplateItem::Literal(current_text.clone()));
                            current_text.clear();
                        }
                        chars.next(); // consume the second $
                        items.push(TemplateItem::Field(FieldPath::new(vec!["$$".to_string()])));
                    } else if next_ch == '{' {
                        // ${name} format
                        // If we have accumulated text, add it as a literal first
                        if !current_text.is_empty() {
                            items.push(TemplateItem::Literal(current_text.clone()));
                            current_text.clear();
                        }
                        chars.next(); // consume the {
                        let mut field_content = String::new();
                        while let Some(&brace_ch) = chars.peek() {
                            if brace_ch == '}' {
                                chars.next(); // consume the closing brace
                                break;
                            }
                            field_content.push(chars.next().unwrap());
                        }
                        let field_path = Self::parse_field_name(&field_content);
                        items.push(TemplateItem::Field(field_path));
                    } else if next_ch.is_ascii_digit() {
                        // $1, $2, etc. format - but check if it looks like a price
                        let mut number = String::new();
                        while let Some(&digit_ch) = chars.peek() {
                            if digit_ch.is_ascii_digit() {
                                number.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }

                        // If the number is too large (like $100), treat as literal
                        if number.len() > 2 {
                            current_text.push('$');
                            current_text.push_str(&number);
                        } else {
                            // If we have accumulated text, add it as a literal first
                            if !current_text.is_empty() {
                                items.push(TemplateItem::Literal(current_text.clone()));
                                current_text.clear();
                            }
                            // Convert 1-based user index to 0-based internal field name
                            // $1 -> field_0, $2 -> field_1, etc.
                            if let Ok(index) = number.parse::<usize>() {
                                if index > 0 {
                                    let field_path =
                                        FieldPath::new(vec![format!("field_{}", index - 1)]);
                                    items.push(TemplateItem::Field(field_path));
                                } else {
                                    // $0 doesn't make sense, treat as literal
                                    current_text.push('$');
                                    current_text.push_str(&number);
                                }
                            } else {
                                // Invalid number, treat as literal
                                current_text.push('$');
                                current_text.push_str(&number);
                            }
                        }
                    } else if next_ch.is_alphabetic() || next_ch == '_' {
                        // $name format
                        // If we have accumulated text, add it as a literal first
                        if !current_text.is_empty() {
                            items.push(TemplateItem::Literal(current_text.clone()));
                            current_text.clear();
                        }
                        let mut field_name = String::new();
                        while let Some(&name_ch) = chars.peek() {
                            if name_ch.is_alphanumeric() || name_ch == '_' || name_ch == '.' {
                                field_name.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }
                        let field_path = Self::parse_field_name(&field_name);
                        items.push(TemplateItem::Field(field_path));
                    } else {
                        // Just a literal $
                        current_text.push('$');
                    }
                } else {
                    // $ at end of string, treat as literal
                    current_text.push('$');
                }
            } else {
                current_text.push(ch);
            }
        }

        // Add any remaining text as a literal
        if !current_text.is_empty() {
            items.push(TemplateItem::Literal(current_text));
        }

        items
    }

    fn parse_field_name(field_name: &str) -> FieldPath {
        let parts: Vec<String> = field_name
            .split('.')
            .map(|s| s.trim().to_string())
            .collect();
        FieldPath::new(parts)
    }
}

// Helper function to parse a complete command line
pub fn parse_command(input: &str) -> Result<ParsedDSL, Box<dyn std::error::Error>> {
    // Handle cases like: "name == 'Alice'" or "name == 'Alice' $name $age"
    // We might need to split the input if it contains both filter and template

    // Simple heuristic: if there are $ not in quotes, split on them
    let parts = split_filter_and_template(input);

    let full_input = if parts.len() == 2 {
        // parts[1] is the template content (already contains the $)
        format!("{} {}", parts[0], parts[1])
    } else {
        input.to_string()
    };

    DSLParser::parse_dsl(&full_input).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

fn split_filter_and_template(input: &str) -> Vec<&str> {
    // Look for the first $ that starts the template
    if let Some(dollar_pos) = input.find('$') {
        let filter_part = input[..dollar_pos].trim();
        let template_part = &input[dollar_pos..]; // Don't trim template - whitespace might be significant
        vec![filter_part, template_part]
    } else {
        vec![input]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let result = parse_command(r#"name == "Alice" $name is $age"#).unwrap();
        assert!(result.filter.is_some());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 3);

        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["name"]),
            _ => panic!("Expected field"),
        }

        match &template.items[1] {
            TemplateItem::Literal(text) => assert_eq!(text, " is "),
            _ => panic!("Expected literal"),
        }

        match &template.items[2] {
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
        let result = parse_command(r#"$$ "#).unwrap();
        assert!(result.filter.is_none());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 2);

        match &template.items[0] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["$$"]),
            _ => panic!("Expected $$ field"),
        }

        match &template.items[1] {
            TemplateItem::Literal(text) => assert_eq!(text, " "),
            _ => panic!("Expected literal space"),
        }
    }

    #[test]
    fn test_template_indexed_fields() {
        let result = parse_command(r#"$1, $2, $3"#).unwrap();
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
        let result = parse_command(r#"User: ${user.name}, Age: ${user.age}"#).unwrap();
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
            parse_command(r#"name == "Alice" Record: $$ - Name: $name, Field1: $1"#).unwrap();
        assert!(result.filter.is_some());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 6);

        match &template.items[0] {
            TemplateItem::Literal(text) => assert_eq!(text, "Record: "),
            _ => panic!("Expected literal"),
        }

        match &template.items[1] {
            TemplateItem::Field(field) => assert_eq!(field.parts, vec!["$$"]),
            _ => panic!("Expected $$ field"),
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
        let result = parse_command(r#"Cost: $100"#).unwrap();
        assert!(result.filter.is_none());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        assert_eq!(template.items.len(), 1);

        match &template.items[0] {
            TemplateItem::Literal(text) => assert_eq!(text, "Cost: $100"),
            _ => panic!("Expected literal with $100"),
        }
    }
}
