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
    },
    OperatorDef {
        symbol: "!=",
        op: ComparisonOp::NotEqual,
    },
    OperatorDef {
        symbol: "<=",
        op: ComparisonOp::LessThanOrEqual,
    },
    OperatorDef {
        symbol: ">=",
        op: ComparisonOp::GreaterThanOrEqual,
    },
    OperatorDef {
        symbol: "^=",
        op: ComparisonOp::StartsWith,
    },
    OperatorDef {
        symbol: "$=",
        op: ComparisonOp::EndsWith,
    },
    OperatorDef {
        symbol: "*=",
        op: ComparisonOp::Contains,
    },
    OperatorDef {
        symbol: "~=",
        op: ComparisonOp::Regex,
    },
    // Single-character operators
    OperatorDef {
        symbol: "<",
        op: ComparisonOp::LessThan,
    },
    OperatorDef {
        symbol: ">",
        op: ComparisonOp::GreaterThan,
    },
];

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Get all operator symbols for use in grammar generation or documentation.
    fn get_all_operator_symbols() -> Vec<&'static str> {
        OPERATORS.iter().map(|op| op.symbol).collect()
    }

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

        // Test fallback
        assert_eq!(parse_comparison_op("unknown"), ComparisonOp::Equal);
    }

    #[test]
    fn test_get_operator_symbols() {
        let symbols = get_all_operator_symbols();
        assert!(symbols.contains(&"=="));
        assert!(symbols.contains(&"*="));
        assert!(symbols.contains(&">="));
        assert_eq!(symbols.len(), OPERATORS.len());
    }
}
