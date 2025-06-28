//! Centralized operator definitions for the parsm DSL.
//!
//! This module provides a single source of truth for all comparison operators
//! supported by parsm, eliminating duplication across the codebase.

use crate::filter::ComparisonOp;

/// Operator definition with string representation and enum variant.
#[derive(Debug, Clone, PartialEq)]
pub struct OperatorDef {
    /// The string representation used in the DSL
    pub symbol: &'static str,
    /// The corresponding enum variant
    pub op: ComparisonOp,
    /// Whether this operator requires spaces around it for disambiguation
    pub needs_spaces: bool,
}

/// All supported comparison operators in order of precedence.
///
/// Longer operators (like ">=") should come before shorter ones ("<")
/// to ensure proper parsing precedence.
pub const OPERATORS: &[OperatorDef] = &[
    // Multi-character operators first (for proper parsing)
    OperatorDef {
        symbol: "==",
        op: ComparisonOp::Equal,
        needs_spaces: false,
    },
    OperatorDef {
        symbol: "!=",
        op: ComparisonOp::NotEqual,
        needs_spaces: false,
    },
    OperatorDef {
        symbol: "<=",
        op: ComparisonOp::LessThanOrEqual,
        needs_spaces: false,
    },
    OperatorDef {
        symbol: ">=",
        op: ComparisonOp::GreaterThanOrEqual,
        needs_spaces: false,
    },
    OperatorDef {
        symbol: "^=",
        op: ComparisonOp::StartsWith,
        needs_spaces: false,
    },
    OperatorDef {
        symbol: "$=",
        op: ComparisonOp::EndsWith,
        needs_spaces: false,
    },
    OperatorDef {
        symbol: "*=",
        op: ComparisonOp::Contains,
        needs_spaces: false,
    },
    // Single-character operators
    OperatorDef {
        symbol: "<",
        op: ComparisonOp::LessThan,
        needs_spaces: true,
    },
    OperatorDef {
        symbol: ">",
        op: ComparisonOp::GreaterThan,
        needs_spaces: true,
    },
    OperatorDef {
        symbol: "~",
        op: ComparisonOp::Matches,
        needs_spaces: false,
    },
];

/// Logical operators for conditions.
pub const LOGICAL_OPERATORS: &[&str] = &["&&", "||", "!"];

/// Parse a comparison operator from its string representation.
///
/// Returns the corresponding `ComparisonOp` variant, or `ComparisonOp::Equal`
/// as a default fallback for unknown operators.
pub fn parse_comparison_op(op_str: &str) -> ComparisonOp {
    OPERATORS
        .iter()
        .find(|op| op.symbol == op_str)
        .map(|op| op.op.clone())
        .unwrap_or(ComparisonOp::Equal)
}

/// Check if the input string contains any filter operators.
///
/// This function is used for disambiguation between field selectors,
/// filters, and templates in the DSL parsing logic.
pub fn contains_filter_operators(input: &str) -> bool {
    // Check comparison operators first
    for op_def in OPERATORS {
        if op_def.needs_spaces {
            // For single-char operators, require spaces to avoid false positives
            let spaced_op = format!(" {} ", op_def.symbol);
            if input.contains(&spaced_op) || input.contains(op_def.symbol) {
                return true;
            }
        } else if input.contains(op_def.symbol) {
            return true;
        }
    }

    // Check logical operators (symbol-based only)
    for logical_op in LOGICAL_OPERATORS {
        match *logical_op {
            "&&" | "||" => {
                if input.contains(logical_op) {
                    return true;
                }
            }
            "!" => {
                if input.starts_with("!") || input.contains(" ! ") {
                    return true;
                }
            }
            _ => {
                // Fallback for any other logical operators
                if input.contains(logical_op) {
                    return true;
                }
            }
        }
    }

    false
}

/// Get all operator symbols for use in grammar generation or documentation.
pub fn get_all_operator_symbols() -> Vec<&'static str> {
    OPERATORS.iter().map(|op| op.symbol).collect()
}

/// Get all logical operator symbols.
pub fn get_all_logical_symbols() -> Vec<&'static str> {
    LOGICAL_OPERATORS.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_comparison_op() {
        // Symbol-based operators from the grammar
        assert_eq!(parse_comparison_op("=="), ComparisonOp::Equal);
        assert_eq!(parse_comparison_op("!="), ComparisonOp::NotEqual);
        assert_eq!(parse_comparison_op("<"), ComparisonOp::LessThan);
        assert_eq!(parse_comparison_op("<="), ComparisonOp::LessThanOrEqual);
        assert_eq!(parse_comparison_op(">"), ComparisonOp::GreaterThan);
        assert_eq!(parse_comparison_op(">="), ComparisonOp::GreaterThanOrEqual);
        assert_eq!(parse_comparison_op("*="), ComparisonOp::Contains);
        assert_eq!(parse_comparison_op("^="), ComparisonOp::StartsWith);
        assert_eq!(parse_comparison_op("$="), ComparisonOp::EndsWith);
        assert_eq!(parse_comparison_op("~"), ComparisonOp::Matches);

        // Test fallback
        assert_eq!(parse_comparison_op("unknown"), ComparisonOp::Equal);
    }

    #[test]
    fn test_contains_filter_operators() {
        // Comparison operators (symbol-based only)
        assert!(contains_filter_operators("age > 25"));
        assert!(contains_filter_operators("name == \"test\""));
        assert!(contains_filter_operators("field != null"));
        assert!(contains_filter_operators("score >= 90"));
        assert!(contains_filter_operators("name *= something"));
        assert!(contains_filter_operators("name ^= prefix"));
        assert!(contains_filter_operators("path $= .txt"));
        assert!(contains_filter_operators("text ~ pattern"));

        // Logical operators
        assert!(contains_filter_operators("active && enabled"));
        assert!(contains_filter_operators("status || fallback"));
        assert!(contains_filter_operators("!active"));

        // Non-operators
        assert!(!contains_filter_operators("simple_field"));
        assert!(!contains_filter_operators("{template}"));
        assert!(!contains_filter_operators("$variable"));
        assert!(!contains_filter_operators("user.name"));
    }

    #[test]
    fn test_get_operator_symbols() {
        let symbols = get_all_operator_symbols();
        assert!(symbols.contains(&"=="));
        assert!(symbols.contains(&"*="));
        assert!(symbols.contains(&">="));
        assert_eq!(symbols.len(), OPERATORS.len());
    }

    #[test]
    fn test_get_logical_symbols() {
        let symbols = get_all_logical_symbols();
        assert!(symbols.contains(&"&&"));
        assert!(symbols.contains(&"||"));
        assert!(symbols.contains(&"!"));
        assert_eq!(symbols.len(), LOGICAL_OPERATORS.len());
    }
}
