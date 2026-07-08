use std::ops::Range;

use jolt_diagnostics::Diagnostic;
use jolt_text::TextRange;

use crate::{Language, SyntaxTrivia};

/// A lexed token whose trivia lives in a caller-owned buffer.
#[derive(Debug, Eq, PartialEq)]
pub struct LexedToken<L: Language> {
    pub kind: L::Kind,
    pub range: TextRange,
    pub leading: Range<usize>,
    pub trailing: Range<usize>,
}

/// Lexer trait that produces [`LexedToken`]s for a specific [`Language`].
///
/// Each language crate implements this on its lexer type and exposes it via
/// `Language::Lexer`.
pub trait LanguageLexer<'source> {
    /// The language this lexer produces tokens for.
    type Language: Language;

    /// Creates a new lexer over the given source text.
    fn new(source: &'source str) -> Self;

    /// Lexes the next token, appending leading/trailing trivia to the
    /// supplied buffer and returning the token's kind, range, and trivia
    /// positions relative to that buffer.
    fn next_token_into(&mut self, trivia: &mut Vec<SyntaxTrivia>) -> LexedToken<Self::Language>;

    /// Consumes the lexer and returns any diagnostics accumulated during
    /// lexing (e.g. unterminated comments, invalid escape sequences).
    fn finish(self) -> Vec<Diagnostic>;
}
