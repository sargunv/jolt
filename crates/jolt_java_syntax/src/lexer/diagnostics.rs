//! The lexer's diagnostics.

use jolt_syntax::{Diagnostic, TextRange};

/// A lexer diagnostic: its code and message.
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

    /// The stable diagnostic code.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::MalformedUnicodeEscape => "java.lex.malformed_unicode_escape",
            Self::UnterminatedBlockComment => "java.lex.unterminated_block_comment",
            Self::UnterminatedCharacterLiteral => "java.lex.unterminated_character_literal",
            Self::UnterminatedStringLiteral => "java.lex.unterminated_string_literal",
            Self::UnterminatedTextBlock => "java.lex.unterminated_text_block",
            Self::MissingTextBlockLineTerminator => "java.lex.missing_text_block_line_terminator",
            Self::InvalidCharacterLiteral => "java.lex.invalid_character_literal",
            Self::InvalidEscapeSequence => "java.lex.invalid_escape_sequence",
            Self::InvalidNumericLiteral => "java.lex.invalid_numeric_literal",
            Self::UnknownCharacter => "java.lex.unknown_character",
        }
    }

    /// The diagnostic at `range`.
    pub(crate) fn at(self, range: TextRange) -> Diagnostic {
        Diagnostic {
            code: self.id(),
            message: self.message().to_owned(),
            range,
        }
    }
}
