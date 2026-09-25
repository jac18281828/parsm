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
    OperatorDef {
        symbol: "~",
        op: ComparisonOp::Contains,
    },
];

/// Parse a comparison operator from its string representation.
///
/// The grammar's `comparison_op` rule only ever matches one of `OPERATORS`'
/// symbols, so a caller passing grammar-matched text never sees `Err` here;
/// an unknown symbol is still an error, never a silent `Equal` fallback.
pub fn parse_comparison_op(op_str: &str) -> Result<ComparisonOp, String> {
    OPERATORS
        .iter()
        .find(|op| op.symbol == op_str)
        .map(|op| op.op.clone())
        .ok_or_else(|| format!("unknown comparison operator '{op_str}'"))
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
        assert_eq!(parse_comparison_op("==").unwrap(), ComparisonOp::Equal);
        assert_eq!(parse_comparison_op("!=").unwrap(), ComparisonOp::NotEqual);
        assert_eq!(parse_comparison_op("<").unwrap(), ComparisonOp::LessThan);
        assert_eq!(
            parse_comparison_op("<=").unwrap(),
            ComparisonOp::LessThanOrEqual
        );
        assert_eq!(parse_comparison_op(">").unwrap(), ComparisonOp::GreaterThan);
        assert_eq!(
            parse_comparison_op(">=").unwrap(),
            ComparisonOp::GreaterThanOrEqual
        );
        assert_eq!(parse_comparison_op("*=").unwrap(), ComparisonOp::Contains);
        assert_eq!(parse_comparison_op("^=").unwrap(), ComparisonOp::StartsWith);
        assert_eq!(parse_comparison_op("$=").unwrap(), ComparisonOp::EndsWith);

        // An unknown symbol is an error, never a silent `Equal` fallback.
        assert!(parse_comparison_op("unknown").is_err());
    }

    /// Every symbol in the operator table parses to its own variant, and an
    /// unknown symbol is always an error.
    #[test]
    fn every_operator_symbol_parses_and_unknown_errors() {
        for op in OPERATORS {
            assert_eq!(parse_comparison_op(op.symbol).unwrap(), op.op);
        }
        assert!(parse_comparison_op("=").is_err());
        assert!(parse_comparison_op("<>").is_err());
        assert!(parse_comparison_op("").is_err());
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
