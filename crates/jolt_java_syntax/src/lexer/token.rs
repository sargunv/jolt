use jolt_text::TextRange;

use super::JavaSyntaxKind;

/// Trivia attached to a token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub range: TextRange,
}

/// The kind of trivia attached to a token.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TriviaKind {
    Whitespace,
    Newline,
    LineComment,
    BlockComment,
    JavadocComment,
    Ignored,
}

/// A lexed Java token with attached trivia and raw source range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: JavaSyntaxKind,
    pub range: TextRange,
    pub leading: Vec<Trivia>,
    pub trailing: Vec<Trivia>,
}

/// The result of lexing Java source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<LexerDiagnostic>,
}

/// A lexer diagnostic with a raw source range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexerDiagnostic {
    pub kind: LexerDiagnosticKind,
    pub range: TextRange,
}

/// The kind of lexer diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LexerDiagnosticKind {
    MalformedUnicodeEscape,
    UnterminatedBlockComment,
    UnterminatedCharacterLiteral,
    UnterminatedStringLiteral,
    UnterminatedTextBlock,
    MissingTextBlockLineTerminator,
    InvalidCharacterLiteral,
    InvalidEscapeSequence,
    InvalidNumericLiteral,
    UnknownCharacter,
}
