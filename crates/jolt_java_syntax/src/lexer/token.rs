use jolt_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticCodeId};
use jolt_text::TextRange;

use crate::JavaSyntaxKind;

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

/// A lexer diagnostic with a raw source range.
pub type LexerDiagnostic = Diagnostic;

/// Stable Java lexer diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaLexDiagnosticCode {
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

impl JavaLexDiagnosticCode {
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::MalformedUnicodeEscape => "malformed Unicode escape",
            Self::UnterminatedBlockComment => "unterminated block comment",
            Self::UnterminatedCharacterLiteral => "unterminated character literal",
            Self::UnterminatedStringLiteral => "unterminated string literal",
            Self::UnterminatedTextBlock => "unterminated text block",
            Self::MissingTextBlockLineTerminator => {
                "text block opening delimiter must be followed by a line terminator"
            }
            Self::InvalidCharacterLiteral => "invalid character literal",
            Self::InvalidEscapeSequence => "invalid escape sequence",
            Self::InvalidNumericLiteral => "invalid numeric literal",
            Self::UnknownCharacter => "unknown character",
        }
    }
}

impl DiagnosticCode for JavaLexDiagnosticCode {
    fn id(&self) -> DiagnosticCodeId {
        match self {
            Self::MalformedUnicodeEscape => {
                DiagnosticCodeId::new("java.lex.malformed_unicode_escape")
            }
            Self::UnterminatedBlockComment => {
                DiagnosticCodeId::new("java.lex.unterminated_block_comment")
            }
            Self::UnterminatedCharacterLiteral => {
                DiagnosticCodeId::new("java.lex.unterminated_character_literal")
            }
            Self::UnterminatedStringLiteral => {
                DiagnosticCodeId::new("java.lex.unterminated_string_literal")
            }
            Self::UnterminatedTextBlock => {
                DiagnosticCodeId::new("java.lex.unterminated_text_block")
            }
            Self::MissingTextBlockLineTerminator => {
                DiagnosticCodeId::new("java.lex.missing_text_block_line_terminator")
            }
            Self::InvalidCharacterLiteral => {
                DiagnosticCodeId::new("java.lex.invalid_character_literal")
            }
            Self::InvalidEscapeSequence => {
                DiagnosticCodeId::new("java.lex.invalid_escape_sequence")
            }
            Self::InvalidNumericLiteral => {
                DiagnosticCodeId::new("java.lex.invalid_numeric_literal")
            }
            Self::UnknownCharacter => DiagnosticCodeId::new("java.lex.unknown_character"),
        }
    }
}
