//! Abstract Syntax Tree definitions for the DSL

use crate::filter::{FieldPath, FilterExpr, Template};

/// Parsed DSL result containing optional filter, template, and field selector.
///
/// This structure represents the parsed result of a user command, which may contain
/// any combination of filtering logic, output templates, and field selection.
/// The relaxed parser ensures at least one component is successfully parsed.
///
/// ## Examples
///
/// ```
/// # use parsm::parse_command;
/// // Field selector only
/// let result = parse_command("name").unwrap();
/// assert!(result.field_selector.is_some());
///
/// // Filter only  
/// let result = parse_command("age > 25").unwrap();
/// assert!(result.filter.is_some());
///
/// // Template only
/// let result = parse_command("{${name}}").unwrap();
/// assert!(result.template.is_some());
///
/// // Combined filter + template
/// let result = parse_command("age > 25 {${name}}").unwrap();
/// assert!(result.filter.is_some() && result.template.is_some());
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{ComparisonOp, FilterExpr, FilterValue, TemplateItem};

    #[test]
    fn test_parsed_dsl_new() {
        let parsed = ParsedDSL::new();
        assert!(parsed.filter.is_none());
        assert!(parsed.template.is_none());
        assert!(parsed.field_selector.is_none());
    }

    #[test]
    fn test_parsed_dsl_default() {
        let parsed = ParsedDSL::default();
        assert!(parsed.filter.is_none());
        assert!(parsed.template.is_none());
        assert!(parsed.field_selector.is_none());
    }

    #[test]
    fn test_parsed_dsl_with_components() {
        let mut parsed = ParsedDSL::new();

        // Test with filter
        parsed.filter = Some(FilterExpr::Comparison {
            field: crate::filter::FieldPath::new(vec!["name".to_string()]),
            op: ComparisonOp::Equal,
            value: FilterValue::String("Alice".to_string()),
        });
        assert!(parsed.filter.is_some());

        // Test with template
        parsed.template = Some(crate::filter::Template {
            items: vec![TemplateItem::Literal("Hello".to_string())],
        });
        assert!(parsed.template.is_some());

        // Test with field selector
        parsed.field_selector = Some(crate::filter::FieldPath::new(vec!["name".to_string()]));
        assert!(parsed.field_selector.is_some());
    }
}
