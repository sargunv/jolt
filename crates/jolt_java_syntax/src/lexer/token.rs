use jolt_syntax::Diagnostic;

/// A lexer diagnostic with a raw source range.
pub(crate) type LexerDiagnostic = Diagnostic;

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
}
