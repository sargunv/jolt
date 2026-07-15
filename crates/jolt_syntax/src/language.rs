use jolt_diagnostics::DiagnosticCodeId;

use crate::LexedToken;
use crate::RawSyntaxKind;

/// A language binding for shared syntax tree infrastructure.
pub trait Language: 'static {
    /// The language-specific syntax kind enum.
    type Kind: Copy + Eq;

    /// The lexer type that produces tokens for this language.
    type Lexer<'source>: crate::LanguageLexer<'source, Language = Self>;

    /// Opaque capability held by the language syntax crate and required to
    /// authorize formatter normalizations.
    type NormalizationAuthority: Copy;

    /// Estimates the parser event capacity from source length.
    ///
    /// Languages whose physical grammar produces denser event streams should
    /// override this so realistic files do not cross a `Vec` growth boundary.
    #[must_use]
    fn initial_event_capacity(source_len: usize) -> usize {
        source_len.div_ceil(2).max(8)
    }

    /// Estimates the syntax-token capacity from source length.
    #[must_use]
    fn initial_token_capacity(source_len: usize) -> usize {
        source_len.div_ceil(8).max(8)
    }

    /// Estimates the trivia-piece capacity from source length.
    #[must_use]
    fn initial_trivia_capacity(source_len: usize) -> usize {
        source_len.div_ceil(12).max(8)
    }

    /// Converts a raw kind stored in shared syntax data to a language kind.
    fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind;

    /// Converts a language kind to the raw representation used by shared syntax data.
    fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind;

    /// Returns the kind that marks end-of-file.
    fn eof_kind() -> Self::Kind;

    /// Returns the kind that marks an error/recovery node.
    fn error_node_kind() -> Self::Kind;

    /// Returns the diagnostic code id for "expected syntax" parser errors.
    fn expected_diagnostic_code() -> DiagnosticCodeId;

    /// Returns the diagnostic code id for "unexpected syntax" parser errors.
    fn unexpected_diagnostic_code() -> DiagnosticCodeId;

    /// If the given lexed token should be split into multiple syntax tokens
    /// (e.g. Java's `>>` into two `>` tokens for type-argument recovery, or
    /// Kotlin's `?.` into `?` and `.`), returns the split kinds.
    fn split_token(token: &LexedToken<Self>) -> Option<&'static [Self::Kind]>
    where
        Self: Sized;
}
