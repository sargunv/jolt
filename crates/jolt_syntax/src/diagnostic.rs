//! Syntax errors.

use crate::TextRange;

/// A lexer or parser error. There is no severity, stage, or owner: every
/// diagnostic is a syntax error at a source range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// A stable code, e.g. `java.parse.expected_syntax`.
    pub code: &'static str,
    pub message: String,
    pub range: TextRange,
}
