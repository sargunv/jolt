use jolt_diagnostics::{Diagnostic, DiagnosticCodeId};

/// A lexer diagnostic with a raw source range.
pub type LexerDiagnostic = Diagnostic;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KotlinLexDiagnosticCode {
    UnterminatedBlockComment,
    UnterminatedBacktickIdentifier,
    UnterminatedCharacterLiteral,
    UnterminatedStringLiteral,
    UnterminatedRawStringLiteral,
    InvalidEscapeSequence,
    UnknownCharacter,
}

impl KotlinLexDiagnosticCode {
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::UnterminatedBlockComment => "unterminated block comment",
            Self::UnterminatedBacktickIdentifier => "unterminated backtick identifier",
            Self::UnterminatedCharacterLiteral => "unterminated character literal",
            Self::UnterminatedStringLiteral => "unterminated string literal",
            Self::UnterminatedRawStringLiteral => "unterminated raw string literal",
            Self::InvalidEscapeSequence => "invalid escape sequence",
            Self::UnknownCharacter => "unknown character",
        }
    }

    #[must_use]
    pub const fn id(self) -> DiagnosticCodeId {
        match self {
            Self::UnterminatedBlockComment => {
                DiagnosticCodeId::new("kotlin.lex.unterminated_block_comment")
            }
            Self::UnterminatedBacktickIdentifier => {
                DiagnosticCodeId::new("kotlin.lex.unterminated_backtick_identifier")
            }
            Self::UnterminatedCharacterLiteral => {
                DiagnosticCodeId::new("kotlin.lex.unterminated_character_literal")
            }
            Self::UnterminatedStringLiteral => {
                DiagnosticCodeId::new("kotlin.lex.unterminated_string_literal")
            }
            Self::UnterminatedRawStringLiteral => {
                DiagnosticCodeId::new("kotlin.lex.unterminated_raw_string_literal")
            }
            Self::InvalidEscapeSequence => {
                DiagnosticCodeId::new("kotlin.lex.invalid_escape_sequence")
            }
            Self::UnknownCharacter => DiagnosticCodeId::new("kotlin.lex.unknown_character"),
        }
    }
}
