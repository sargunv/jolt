//! Spacing between adjacent tokens.

use jolt_syntax::SyntaxToken;

/// Whether two adjacent tokens printed with no separator would lex as
/// different tokens (`- -x`, `+ +x`, `a instanceof`, `/ /`).
pub(crate) fn needs_space(left: SyntaxToken<'_>, right: SyntaxToken<'_>) -> bool {
    let (Some(last), Some(first)) = (left.text().chars().last(), right.text().chars().next())
    else {
        return false;
    };
    let word = |ch: char| ch.is_alphanumeric() || ch == '_' || ch == '$' || ch == '\\';
    (word(last) && word(first))
        || (last == '+' && first == '+')
        || (last == '-' && first == '-')
        || (last == '/' && matches!(first, '/' | '*'))
}
