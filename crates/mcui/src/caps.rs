//! What the calculator holds of one assertion's statements.

use crate::expr::Expr;

/// Statements of an assertion: its essential hypotheses and its conclusion.
pub const MAX_STATEMENTS: usize = 32;
/// Tokens of all an assertion's statements together, each with its typecode.
pub const MAX_TOKENS: usize = 768;
/// Subexpressions of all an assertion's statements together.
pub const MAX_NODES: usize = 384;
/// Tokens of one statement, with its typecode.
pub const MAX_STATEMENT_TOKENS: usize = 255;
/// Subexpressions of one statement.
pub const MAX_STATEMENT_NODES: usize = 255;

/// Whether one statement is inside the per-statement caps.
pub fn statement_within(expr: &Expr) -> bool {
    expr.tokens().len() <= MAX_STATEMENT_TOKENS && expr.len() <= MAX_STATEMENT_NODES
}

/// Whether an assertion's statements, hypotheses first and the conclusion last, are inside every
/// cap.
pub fn within(statements: &[&Expr]) -> bool {
    let tokens: usize = statements.iter().map(|e| e.tokens().len()).sum();
    let nodes: usize = statements.iter().map(|e| e.len()).sum();
    statements.len() <= MAX_STATEMENTS
        && tokens <= MAX_TOKENS
        && nodes <= MAX_NODES
        && statements.iter().all(|e| statement_within(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(tokens: usize) -> Expr {
        let tokens: Vec<String> = vec!["zz".to_string(); tokens - 1];
        Expr::flat(vec!["|-".to_string()], &tokens)
    }

    #[test]
    fn a_statement_is_inside_at_255_tokens_and_outside_at_256() {
        assert!(statement_within(&flat(255)));
        assert!(!statement_within(&flat(256)));
        assert!(within(&[&flat(255)]));
        assert!(!within(&[&flat(256)]));
    }
}
