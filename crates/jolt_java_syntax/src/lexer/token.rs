use std::ops::Range;

use jolt_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticCodeId};
use jolt_text::TextRange;

use crate::JavaSyntaxKind;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Trivia {
    pub(crate) kind: TriviaKind,
    pub(crate) range: TextRange,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum TriviaKind {
    Whitespace,
    Newline,
    LineComment,
    BlockComment,
    JavadocComment,
    Ignored,
}

#[derive(Debug, Eq, PartialEq)]
#[cfg(test)]
pub(crate) struct Token {
    pub(crate) kind: JavaSyntaxKind,
    pub(crate) range: TextRange,
    pub(crate) leading: Vec<Trivia>,
    pub(crate) trailing: Vec<Trivia>,
}

/// A lexed token whose trivia lives in a caller-owned buffer.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct LexedToken {
    pub(crate) kind: JavaSyntaxKind,
    pub(crate) range: TextRange,
    pub(crate) leading: Range<usize>,
    pub(crate) trailing: Range<usize>,
}

/// A lexer diagnostic with a raw source range.
pub(crate) type LexerDiagnostic = Diagnostic;

/// Stable Java lexer diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum JavaLexDiagnosticCode {
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
    pub(crate) const fn message(self) -> &'static str {
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
