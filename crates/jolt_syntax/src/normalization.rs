use std::marker::PhantomData;

use crate::{Language, SourceIdentity, SourceNodeId, SourceTokenId, SyntaxNode};

/// Proof that one complete syntax owner is valid enough to authorize a
/// semantics-preserving normalization.
pub struct NormalizationOwner<'tree, L: Language> {
    source: SourceNodeId<'tree>,
    language: PhantomData<L>,
}

impl<L: Language> Copy for NormalizationOwner<'_, L> {}

impl<L: Language> Clone for NormalizationOwner<'_, L> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'tree, L: Language> NormalizationOwner<'tree, L> {
    /// Creates an owner proof only for a complete recovery-free syntax node.
    #[doc(hidden)]
    pub fn authorized(
        _authority: L::NormalizationAuthority,
        owner: &SyntaxNode<'tree, L>,
    ) -> Option<Self> {
        owner.is_recovery_free().then_some(Self {
            source: owner.source_id(),
            language: PhantomData,
        })
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn source_id(self) -> SourceNodeId<'tree> {
        self.source
    }

    #[doc(hidden)]
    #[must_use]
    pub fn owns_token(self, token: SourceTokenId<'tree>) -> bool {
        self.source.contains_token(token)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn owns_boundary_token(self, token: SourceTokenId<'tree>) -> bool {
        self.owns_token(token) || self.source.immediately_precedes(token)
    }
}

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

/// A closed semantics-preserving source-order normalization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReorderReason {
    Imports,
    Modifiers,
    ModuleDirectives,
    RequiresModifiers,
}

/// Syntax-authorized permission to reorder one recovery-free syntax owner.
pub struct ReorderClaim<'tree> {
    owner: SourceNodeId<'tree>,
    anchor: SourceTokenId<'tree>,
    reason: ReorderReason,
}

impl<'tree> ReorderClaim<'tree> {
    #[doc(hidden)]
    #[must_use]
    pub fn authorized<L: Language>(
        owner: NormalizationOwner<'tree, L>,
        anchor: SourceTokenId<'tree>,
        reason: ReorderReason,
    ) -> Self {
        assert!(
            owner.owns_token(anchor),
            "normalization anchor must belong to its complete owner"
        );
        Self {
            owner: owner.source_id(),
            anchor,
            reason,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn into_parts(self) -> (SourceTokenId<'tree>, ReorderReason) {
        (self.anchor, self.reason)
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn owner(self) -> SourceNodeId<'tree> {
        self.owner
    }
}

/// Syntax-authorized permission to replace one represented source token.
pub struct ReplacementClaim<'tree> {
    owner: SourceNodeId<'tree>,
    source: SourceTokenId<'tree>,
    token: NormalizedToken,
}

impl<'tree> ReplacementClaim<'tree> {
    #[doc(hidden)]
    #[must_use]
    pub fn authorized<L: Language>(
        owner: NormalizationOwner<'tree, L>,
        source: SourceTokenId<'tree>,
        token: NormalizedToken,
    ) -> Self {
        assert!(
            owner.owns_token(source),
            "replacement source must belong to its complete owner"
        );
        Self {
            owner: owner.source_id(),
            source,
            token,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn into_parts(self) -> (SourceTokenId<'tree>, NormalizedToken) {
        (self.source, self.token)
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn owner(self) -> SourceNodeId<'tree> {
        self.owner
    }
}

/// Syntax-authorized permission to consume represented syntax without output.
pub struct RemovalClaim<'tree> {
    owner: SourceNodeId<'tree>,
    source: SourceIdentity<'tree>,
    reason: RemovalReason,
}

impl<'tree> RemovalClaim<'tree> {
    #[doc(hidden)]
    #[must_use]
    pub fn authorized<L: Language>(
        owner: NormalizationOwner<'tree, L>,
        source: SourceIdentity<'tree>,
        reason: RemovalReason,
    ) -> Self {
        assert!(
            owner.owns_token(source.token_id()),
            "removed source must belong to its complete owner"
        );
        Self {
            owner: owner.source_id(),
            source,
            reason,
        }
    }

    /// Authorizes a represented boundary token contained by, or immediately
    /// following, its complete syntax owner.
    #[doc(hidden)]
    #[must_use]
    pub fn authorized_boundary<L: Language>(
        owner: NormalizationOwner<'tree, L>,
        source: SourceIdentity<'tree>,
        reason: RemovalReason,
    ) -> Self {
        assert!(
            owner.owns_boundary_token(source.token_id()),
            "removed boundary must belong to or immediately follow its complete owner"
        );
        Self {
            owner: owner.source_id(),
            source,
            reason,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn into_parts(self) -> (SourceIdentity<'tree>, RemovalReason) {
        (self.source, self.reason)
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn owner(self) -> SourceNodeId<'tree> {
        self.owner
    }
}

/// Syntax-authorized permission to synthesize one normalization token.
pub struct SynthesisClaim<'tree> {
    owner: SourceNodeId<'tree>,
    anchor: SourceTokenId<'tree>,
    token: NormalizedToken,
}

impl<'tree> SynthesisClaim<'tree> {
    #[doc(hidden)]
    #[must_use]
    pub fn authorized<L: Language>(
        owner: NormalizationOwner<'tree, L>,
        anchor: SourceTokenId<'tree>,
        token: NormalizedToken,
    ) -> Self {
        assert!(
            owner.owns_token(anchor),
            "normalization anchor must belong to its complete owner"
        );
        Self {
            owner: owner.source_id(),
            anchor,
            token,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn into_parts(self) -> (SourceTokenId<'tree>, NormalizedToken) {
        (self.anchor, self.token)
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn owner(self) -> SourceNodeId<'tree> {
        self.owner
    }
}
