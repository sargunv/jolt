#[cfg(debug_assertions)]
use jolt_syntax::SourceIdentity;
use jolt_syntax::{
    ConservationError, Language, NormalizedToken, RawSyntaxKind, RemovalReason, ReorderReason,
    SourceRangeClaim, SourceTokenId, SyntaxConservationTracker, SyntaxToken,
};
use jolt_text::TextRange;

use crate::Doc;

/// The lexical class at one edge of an exceptional source fragment.
///
/// Ordinary structured documents do not carry this metadata. It exists only
/// where source-backed output bypasses normal token formatting and therefore
/// needs a bounded lexical-safety decision at a surrounding join.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LexicalAtomKind {
    Identifier,
    Number,
    String,
    Punctuation,
    Comment,
}

pub(crate) const fn normalized_lexical_kind(token: NormalizedToken) -> LexicalAtomKind {
    match token {
        NormalizedToken::ImportKeyword | NormalizedToken::ImportAliasKeyword => {
            LexicalAtomKind::Identifier
        }
        _ => LexicalAtomKind::Punctuation,
    }
}

/// A borrowed lexical atom at an exceptional fragment boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LexicalAtom<'source> {
    kind: LexicalAtomKind,
    text: &'source str,
}

impl<'source> LexicalAtom<'source> {
    #[must_use]
    pub(crate) const fn new(kind: LexicalAtomKind, text: &'source str) -> Self {
        Self { kind, text }
    }

    #[must_use]
    pub const fn kind(self) -> LexicalAtomKind {
        self.kind
    }

    #[must_use]
    pub const fn text(self) -> &'source str {
        self.text
    }
}

/// Boundary facts needed when joining an exceptional fragment to structured
/// output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FragmentBoundary<'source> {
    pub(crate) first: Option<LexicalAtom<'source>>,
    pub(crate) last: Option<LexicalAtom<'source>>,
    pub(crate) ends_with_line_comment: bool,
}

/// A separator required to keep an exceptional fragment lexically distinct
/// from adjacent structured syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExceptionalSeparator {
    None,
    Space,
    HardLine,
}

/// Language-specific lexical safety for joins involving exceptional source.
///
/// Implementations receive already classified boundary atoms. They must not
/// inspect raw source gaps or tokenize fragment text.
pub trait LexicalSafety<L: Language> {
    fn classify(&mut self, token: &SyntaxToken<'_, L>) -> LexicalAtomKind;

    fn separator(&mut self, left: LexicalAtom<'_>, right: LexicalAtom<'_>) -> ExceptionalSeparator;
}

/// The bounded separator decisions around one exceptional fragment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExceptionalSeparators {
    pub before: ExceptionalSeparator,
    pub after: ExceptionalSeparator,
}

/// Computes lexical separators around an exceptional fragment.
///
/// Each present adjacent atom pair is delegated exactly once. A trailing line
/// comment always forces a hard line before following syntax without consulting
/// language policy.
#[must_use]
pub(crate) fn exceptional_separators<L: Language>(
    left: Option<LexicalAtom<'_>>,
    fragment: ExceptionalFragment<'_>,
    right: Option<LexicalAtom<'_>>,
    safety: &mut impl LexicalSafety<L>,
) -> ExceptionalSeparators {
    let boundary = fragment.boundary();
    let before = match (left, boundary.first) {
        (Some(left), Some(first)) => safety.separator(left, first),
        _ => ExceptionalSeparator::None,
    };
    let after = if boundary.ends_with_line_comment && right.is_some() {
        ExceptionalSeparator::HardLine
    } else {
        match (boundary.last, right) {
            (Some(last), Some(right)) => safety.separator(last, right),
            _ => ExceptionalSeparator::None,
        }
    };
    ExceptionalSeparators { before, after }
}

/// An exceptional document together with the lexical facts needed by a
/// surrounding structured join.
///
/// The boundary is deliberately outside the document arena: ordinary docs and
/// renderer nodes do not pay for metadata used only at exceptional joins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExceptionalFragment<'source> {
    proof: Doc<'source>,
    doc: Doc<'source>,
    boundary: FragmentBoundary<'source>,
}

impl<'source> ExceptionalFragment<'source> {
    pub(crate) const fn new(
        proof: Doc<'source>,
        doc: Doc<'source>,
        boundary: FragmentBoundary<'source>,
    ) -> Self {
        Self {
            proof,
            doc,
            boundary,
        }
    }

    #[must_use]
    pub(crate) const fn proof(self) -> Doc<'source> {
        self.proof
    }

    #[must_use]
    pub(crate) const fn doc(self) -> Doc<'source> {
        self.doc
    }

    #[must_use]
    pub(crate) const fn boundary(self) -> FragmentBoundary<'source> {
        self.boundary
    }
}

/// The exceptional source operation represented by a fragment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceProofKind<'tree> {
    MalformedVerbatim {
        kind: RawSyntaxKind,
        range: TextRange,
    },
    Replaced {
        token: NormalizedToken,
    },
    Removed {
        reason: RemovalReason,
    },
    Synthesized {
        token: NormalizedToken,
        anchor: SourceTokenId<'tree>,
    },
    Reordered {
        reason: ReorderReason,
        anchor: SourceTokenId<'tree>,
    },
    FormatterIgnore {
        range: SourceRangeClaim<'tree>,
    },
}

/// Render-time conservation proof and exceptional-fragment ledger.
///
/// The syntax tracker compiles to zero state in optimized builds. The fragment
/// ledger likewise exists only with debug assertions, so ordinary release
/// formatting adds no tracker or comment-map allocation.
pub(crate) struct RenderProof<'tree> {
    conservation: SyntaxConservationTracker<'tree>,
    #[cfg(debug_assertions)]
    malformed_verbatim_count: usize,
}

impl<'tree> RenderProof<'tree> {
    #[must_use]
    pub(crate) fn new(conservation: SyntaxConservationTracker<'tree>) -> Self {
        Self {
            conservation,
            #[cfg(debug_assertions)]
            malformed_verbatim_count: 0,
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn render_fragment(
        &mut self,
        fragment: &SourceProof<'tree>,
        claims: &[SourceIdentity<'tree>],
    ) -> Result<(), ConservationError> {
        #[cfg(not(debug_assertions))]
        let _ = fragment;
        #[cfg(debug_assertions)]
        if let Some(
            SourceProofKind::Synthesized { anchor, .. } | SourceProofKind::Reordered { anchor, .. },
        ) = fragment.kind
        {
            self.conservation.validate_token(anchor)?;
        }
        if let Some(SourceProofKind::FormatterIgnore { range }) = fragment.kind {
            self.conservation.claim_source_range(range)?;
        }
        for identity in claims {
            self.conservation.claim(*identity)?;
        }
        #[cfg(debug_assertions)]
        if matches!(
            fragment.kind,
            Some(SourceProofKind::MalformedVerbatim { .. })
        ) {
            self.malformed_verbatim_count += 1;
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<bool, ConservationError> {
        self.conservation.finish()?;
        #[cfg(debug_assertions)]
        {
            Ok(self.malformed_verbatim_count != 0)
        }
        #[cfg(not(debug_assertions))]
        {
            Ok(false)
        }
    }
}

#[cfg(debug_assertions)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceProof<'tree> {
    pub(crate) kind: Option<SourceProofKind<'tree>>,
    pub(crate) claims_start: u32,
    pub(crate) claims_len: u32,
}

#[cfg(debug_assertions)]
impl<'tree> SourceProof<'tree> {
    pub(crate) const fn new(
        kind: Option<SourceProofKind<'tree>>,
        claims_start: u32,
        claims_len: u32,
    ) -> Self {
        Self {
            kind,
            claims_start,
            claims_len,
        }
    }
}
