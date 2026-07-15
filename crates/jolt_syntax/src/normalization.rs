use crate::{Language, SourceIdentity, SourceTokenId};

/// A closed, semantics-preserving token normalization used by Jolt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NormalizedToken {
    OpenBlockBrace,
    CloseBlockBrace,
    OpenPrecedenceParenthesis,
    ClosePrecedenceParenthesis,
    TrailingComma,
    EnumComma,
    EnumSemicolon,
    ImportKeyword,
    ImportAliasKeyword,
}

impl NormalizedToken {
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::OpenBlockBrace => "{",
            Self::CloseBlockBrace => "}",
            Self::OpenPrecedenceParenthesis => "(",
            Self::ClosePrecedenceParenthesis => ")",
            Self::TrailingComma | Self::EnumComma => ",",
            Self::EnumSemicolon => ";",
            Self::ImportKeyword => "import",
            Self::ImportAliasKeyword => "as",
        }
    }
}

/// A closed reason for consuming represented syntax without emitting it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemovalReason {
    DuplicateImport,
    RedundantDelimiter,
    RedundantSeparator,
}

/// Syntax-authorized permission to replace one represented source token.
pub struct ReplacementClaim<'tree> {
    source: SourceTokenId<'tree>,
    token: NormalizedToken,
}

impl<'tree> ReplacementClaim<'tree> {
    #[doc(hidden)]
    pub const fn authorized<L: Language>(
        _authority: L::NormalizationAuthority,
        source: SourceTokenId<'tree>,
        token: NormalizedToken,
    ) -> Self {
        Self { source, token }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn into_parts(self) -> (SourceTokenId<'tree>, NormalizedToken) {
        (self.source, self.token)
    }
}

/// Syntax-authorized permission to consume represented syntax without output.
pub struct RemovalClaim<'tree> {
    source: SourceIdentity<'tree>,
    reason: RemovalReason,
}

impl<'tree> RemovalClaim<'tree> {
    #[doc(hidden)]
    pub const fn authorized<L: Language>(
        _authority: L::NormalizationAuthority,
        source: SourceIdentity<'tree>,
        reason: RemovalReason,
    ) -> Self {
        Self { source, reason }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn into_parts(self) -> (SourceIdentity<'tree>, RemovalReason) {
        (self.source, self.reason)
    }
}

/// Syntax-authorized permission to synthesize one normalization token.
pub struct SynthesisClaim<'tree> {
    anchor: SourceTokenId<'tree>,
    token: NormalizedToken,
}

impl<'tree> SynthesisClaim<'tree> {
    #[doc(hidden)]
    pub const fn authorized<L: Language>(
        _authority: L::NormalizationAuthority,
        anchor: SourceTokenId<'tree>,
        token: NormalizedToken,
    ) -> Self {
        Self { anchor, token }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn into_parts(self) -> (SourceTokenId<'tree>, NormalizedToken) {
        (self.anchor, self.token)
    }
}
