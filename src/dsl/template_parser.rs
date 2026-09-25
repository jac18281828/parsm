//! Template expression parser

use pest::iterators::Pair;
use tracing::trace;

use super::grammar::{DSLParser, Rule};
use crate::filter::{FieldPath, Template, TemplateItem};

pub struct TemplateParser;

impl TemplateParser {
    pub fn parse_template_expr(
        pair: Pair<Rule>,
    ) -> Result<Template, Box<pest::error::Error<Rule>>> {
        let inner = match pair.into_inner().next() {
            Some(inner) => inner,
            None => {
                trace!("parse_template_expr: no inner content found");
                return Ok(Template { items: Vec::new() });
            }
        };

        match inner.as_rule() {
            Rule::braced_template => Self::parse_braced_template(inner),
            Rule::bracketed_template => Self::parse_bracketed_template(inner),
            Rule::simple_variable => {
                // simple_variable = @{ "$" ~ non_numeric_field_path }: always
                // starts with "$".
                let var_str = inner.as_str();
                let field_name = var_str.strip_prefix('$').unwrap();
                trace!("Parsing simple_variable: '{}'", var_str);

                // Check if this is a numeric dollar amount (like $20, $0, $1)
                if field_name.chars().all(|c| c.is_ascii_digit()) && !field_name.is_empty() {
                    trace!(
                        "Simple variable '{}' is numeric dollar amount, treating as literal",
                        var_str
                    );
                    // Treat numeric dollar amounts as literals
                    Ok(Template {
                        items: vec![TemplateItem::Literal(var_str.to_string())],
                    })
                } else {
                    trace!(
                        "Simple variable '{}' is field reference, treating as field",
                        var_str
                    );
                    // Treat non-numeric as field substitution
                    let field_path = Self::parse_field_path_from_simple_var(inner);
                    Ok(Template {
                        items: vec![TemplateItem::Field(field_path)],
                    })
                }
            }
            Rule::template_conditional => Ok(Template {
                items: vec![Self::parse_template_conditional(inner)],
            }),
            Rule::braced_variable => {
                // Bare "${field}" at the top level - e.g. "${0}" (mapped to
                // the $0 original-input field, same special-case as when
                // embedded inside "{...}"/"[...]") or "${name}". braced_variable
                // = { "${" ~ field_path ~ "}" }: always exactly one inner pair.
                let field_path_pair = inner.into_inner().next().unwrap();
                let field_path = DSLParser::parse_field_path(field_path_pair);
                let field_path = if field_path.parts.len() == 1 && field_path.parts[0] == "0" {
                    FieldPath::new(vec!["$0".to_string()])
                } else {
                    field_path
                };
                Ok(Template {
                    items: vec![TemplateItem::Field(field_path)],
                })
            }
            Rule::dollar_digits_literal => {
                // Bare "$NNN" (all-digit) at the top level is a literal
                // dollar amount, e.g. "$20" renders as "$20".
                Ok(Template {
                    items: vec![TemplateItem::Literal(inner.as_str().to_string())],
                })
            }
            _ => unreachable!("Unexpected template expression type"),
        }
    }

    /// Build a `TemplateItem::Conditional` from a `template_conditional` pair:
    /// `"${" ~ field_path ~ "?" ~ template_content ~ ":" ~ template_content ~ "}"`.
    fn parse_template_conditional(pair: Pair<Rule>) -> TemplateItem {
        // template_conditional = { "${" ~ field_path ~ "?" ~ template_content
        // ~ ":" ~ template_content ~ "}" }: always these three inner pairs,
        // in this order.
        let mut inner = pair.into_inner();
        let field_path_pair = inner.next().unwrap();
        let field = DSLParser::parse_field_path(field_path_pair);
        let true_content = inner.next().unwrap();
        let false_content = inner.next().unwrap();
        let true_template = Self::parse_template_content_rule(true_content);
        let false_template = Self::parse_template_content_rule(false_content);
        TemplateItem::Conditional {
            field,
            true_template,
            false_template,
        }
    }

    /// Parse a `template_content` pair (`template_item*`, each item a
    /// `template_variable | template_literal`) into a `Template`. Used for the
    /// true/false branches of `${field?a:b}`.
    fn parse_template_content_rule(pair: Pair<Rule>) -> Template {
        let mut items = Vec::new();
        for template_item in pair.into_inner() {
            let inner = match template_item.into_inner().next() {
                Some(inner) => inner,
                None => continue,
            };
            match inner.as_rule() {
                Rule::template_variable => {
                    let field_path = Self::parse_template_variable(inner);
                    items.push(TemplateItem::Field(field_path));
                }
                Rule::template_literal => {
                    let text = inner.as_str().to_string();
                    if !text.is_empty() {
                        items.push(TemplateItem::Literal(text));
                    }
                }
                _ => {}
            }
        }
        Template { items }
    }

    fn parse_braced_template(pair: Pair<Rule>) -> Result<Template, Box<pest::error::Error<Rule>>> {
        // braced_template = "{" ~ braced_template_content ~ "}": braced_template_content
        // is the only rule inside, so this pair is always present, even when empty.
        let template_content = pair.into_inner().next().unwrap();
        Self::parse_template_content_from_pairs(template_content)
    }

    fn parse_bracketed_template(
        pair: Pair<Rule>,
    ) -> Result<Template, Box<pest::error::Error<Rule>>> {
        // bracketed_template = "[" ~ bracketed_template_content ~ "]": same guarantee
        // as parse_braced_template above.
        let template_content = pair.into_inner().next().unwrap();
        Self::parse_template_content_from_pairs(template_content)
    }

    /// Parse a `braced_template_content` or `bracketed_template_content` pair
    /// (each a `*_item*` sequence) into a flat `Template`.
    fn parse_template_content_from_pairs(
        pair: Pair<Rule>,
    ) -> Result<Template, Box<pest::error::Error<Rule>>> {
        let mut items = Vec::new();
        for item in pair.into_inner() {
            Self::push_template_content_item(&mut items, item)?;
        }
        Ok(Template { items })
    }

    /// Resolve one `*_template_item` (or, recursively, `bracket_literal_span`)
    /// pair and append it to `items`: a field, a conditional, a literal run,
    /// or a balanced `[...]` span whose brackets are literal but whose
    /// content still interpolates.
    fn push_template_content_item(
        items: &mut Vec<TemplateItem>,
        item: Pair<Rule>,
    ) -> Result<(), Box<pest::error::Error<Rule>>> {
        match item.as_rule() {
            Rule::braced_template_item | Rule::bracketed_template_item => {
                // template_variable | template_conditional | bracket_literal_span |
                // *_template_literal: the grammar guarantees exactly one inner pair.
                let inner_item = item.into_inner().next().unwrap();
                Self::push_template_content_item(items, inner_item)
            }
            Rule::template_variable => {
                items.push(TemplateItem::Field(Self::parse_template_variable(item)));
                Ok(())
            }
            Rule::template_conditional => {
                items.push(Self::parse_template_conditional(item));
                Ok(())
            }
            Rule::braced_template_literal | Rule::bracketed_template_literal => {
                let text = item.as_str().to_string();
                if !text.is_empty() {
                    items.push(TemplateItem::Literal(text));
                }
                Ok(())
            }
            Rule::bracket_literal_span => {
                // "[" ~ bracketed_template_item* ~ "]": the brackets are literal,
                // content between them still interpolates.
                items.push(TemplateItem::Literal("[".to_string()));
                for inner_item in item.into_inner() {
                    Self::push_template_content_item(items, inner_item)?;
                }
                items.push(TemplateItem::Literal("]".to_string()));
                Ok(())
            }
            Rule::bracket_escape => {
                // bracket_escape = @{ "\\" ~ ("[" | "]") }: the character
                // after the backslash is the literal bracket it escapes.
                let escaped = item.as_str().chars().nth(1).unwrap();
                items.push(TemplateItem::Literal(escaped.to_string()));
                Ok(())
            }
            other => unreachable!("Unexpected template content item: {other:?}"),
        }
    }

    fn parse_template_variable(pair: Pair<Rule>) -> FieldPath {
        trace!("parse_template_variable called with: '{}'", pair.as_str());
        // template_variable = { braced_variable | plain_variable }: always
        // exactly one inner pair.
        let inner = pair.into_inner().next().unwrap();
        trace!(
            "parse_template_variable inner rule: {:?} with text: '{}'",
            inner.as_rule(),
            inner.as_str()
        );

        match inner.as_rule() {
            Rule::braced_variable => {
                // ${field_path} - extract the field_path. braced_variable =
                // { "${" ~ field_path ~ "}" }: always one inner pair.
                trace!("Processing braced_variable: '{}'", inner.as_str());
                let field_path_pair = inner.into_inner().next().unwrap();
                let field_path = DSLParser::parse_field_path(field_path_pair);
                trace!("Braced variable field path: {:?}", field_path.parts);

                // Special case: ${0} should map to the $0 field (original input)
                if field_path.parts.len() == 1 && field_path.parts[0] == "0" {
                    trace!("Converting ${{0}} to $0 field");
                    return FieldPath::new(vec!["$0".to_string()]);
                }

                field_path
            }
            Rule::plain_variable => {
                // $field_path - extract the field_path (no special handling
                // for $0). plain_variable = { "$" ~ non_numeric_field_path }:
                // always one inner pair.
                trace!("Processing plain_variable: '{}'", inner.as_str());
                let field_path_pair = inner.into_inner().next().unwrap();
                let field_path = DSLParser::parse_field_path(field_path_pair);
                trace!("Plain variable field path: {:?}", field_path.parts);
                field_path
            }
            Rule::field_path => {
                // Direct field path
                trace!("Processing direct field_path: '{}'", inner.as_str());
                let field_path = DSLParser::parse_field_path(inner);
                trace!("Direct field path: {:?}", field_path.parts);
                field_path
            }
            _ => unreachable!("Unexpected template variable type"),
        }
    }

    fn parse_field_path_from_simple_var(pair: Pair<Rule>) -> FieldPath {
        // simple_variable = @{ "$" ~ non_numeric_field_path }: always
        // starts with "$".
        let var_str = pair.as_str();
        trace!(
            "parse_field_path_from_simple_var called with atomic rule: '{}'",
            var_str
        );
        let field_name = var_str.strip_prefix('$').unwrap();
        let parts: Vec<String> = field_name.split('.').map(|s| s.to_string()).collect();
        FieldPath::new(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::TemplateItem;

    /// Parse `input` as a template through the grammar's own entry point -
    /// templates are built from pest pairs only, never a raw-string reparse.
    fn template_of(input: &str) -> Template {
        DSLParser::parse_dsl(input)
            .unwrap_or_else(|e| panic!("'{input}' failed to parse: {e}"))
            .template
            .unwrap_or_else(|| panic!("'{input}' did not parse as a template"))
    }

    #[test]
    fn interpolated_literal_stops_at_the_variable_boundary() {
        // "$name." is the field "name" followed by a literal ".", never
        // "name." as one variable name.
        let template = template_of("[Hello $name.]");
        assert_eq!(
            template.items,
            vec![
                TemplateItem::Literal("Hello ".to_string()),
                TemplateItem::Field(FieldPath::new(vec!["name".to_string()])),
                TemplateItem::Literal(".".to_string()),
            ]
        );
    }

    #[test]
    fn braced_variable_zero_maps_to_source() {
        let template = template_of("{${0}}");
        assert_eq!(
            template.items,
            vec![TemplateItem::Field(FieldPath::new(vec!["$0".to_string()]))]
        );
    }

    #[test]
    fn braced_variable_numeric_field_stays_as_is() {
        let template = template_of("{${1}}");
        assert_eq!(
            template.items,
            vec![TemplateItem::Field(FieldPath::new(vec!["1".to_string()]))]
        );
    }

    #[test]
    fn mixed_literal_and_field_content() {
        let template = template_of("[ID: ${user_id}, Amount: $20, Name: ${name}]");
        assert_eq!(
            template.items,
            vec![
                TemplateItem::Literal("ID: ".to_string()),
                TemplateItem::Field(FieldPath::new(vec!["user_id".to_string()])),
                TemplateItem::Literal(", Amount: $20, Name: ".to_string()),
                TemplateItem::Field(FieldPath::new(vec!["name".to_string()])),
            ]
        );
    }

    #[test]
    fn nested_field_path_variable() {
        let template = template_of("{${user.profile.name}}");
        assert_eq!(
            template.items,
            vec![TemplateItem::Field(FieldPath::new(vec![
                "user".to_string(),
                "profile".to_string(),
                "name".to_string(),
            ]))]
        );
    }

    #[test]
    fn empty_braces_and_brackets_are_empty_templates() {
        assert_eq!(template_of("{}").items, Vec::new());
        assert_eq!(template_of("[]").items, Vec::new());
    }

    #[test]
    fn literal_only_content() {
        let template = template_of("{Hello World}");
        assert_eq!(
            template.items,
            vec![TemplateItem::Literal("Hello World".to_string())]
        );
    }

    #[test]
    fn balanced_bracket_span_is_literal_but_still_interpolates() {
        // A nested "[...]" pair inside a bracketed template is literal text;
        // the variable inside it still interpolates.
        let template = template_of("[[${level}] ${msg}]");
        assert_eq!(
            template.items,
            vec![
                TemplateItem::Literal("[".to_string()),
                TemplateItem::Field(FieldPath::new(vec!["level".to_string()])),
                TemplateItem::Literal("]".to_string()),
                TemplateItem::Literal(" ".to_string()),
                TemplateItem::Field(FieldPath::new(vec!["msg".to_string()])),
            ]
        );
    }

    #[test]
    fn braced_template_accepts_conditionals() {
        let template = template_of("{${a?x:y}}");
        assert!(matches!(
            template.items[0],
            TemplateItem::Conditional { .. }
        ));
    }
}
