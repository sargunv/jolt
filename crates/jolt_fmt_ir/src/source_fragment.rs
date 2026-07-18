use jolt_syntax::{
    ConservationError, Language, NormalizedToken, RawSyntaxKind, RemovalReason, ReorderReason,
    SourceIdentity, SourceRangeClaim, SourceTokenId, SyntaxConservationTracker, SyntaxToken,
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
    doc: Doc<'source>,
    boundary: FragmentBoundary<'source>,
}

impl<'source> ExceptionalFragment<'source> {
    pub(crate) const fn new(doc: Doc<'source>, boundary: FragmentBoundary<'source>) -> Self {
        Self { doc, boundary }
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
pub(crate) enum SourceClaim<'tree> {
    Identity(SourceIdentity<'tree>),
    MalformedVerbatim {
        claim: SourceRangeClaim<'tree>,
        kind: RawSyntaxKind,
        range: TextRange,
    },
    Replaced {
        source: SourceTokenId<'tree>,
        token: NormalizedToken,
    },
    Removed {
        source: SourceIdentity<'tree>,
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

/// Render-time checker for the claims carried by the selected document branch.
pub(crate) struct RenderProof<'tree> {
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    conservation: SyntaxConservationTracker<'tree>,
    #[cfg(debug_assertions)]
    malformed_verbatim_count: usize,
}

impl<'tree> RenderProof<'tree> {
    #[must_use]
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub(crate) fn new(conservation: SyntaxConservationTracker<'tree>) -> Self {
        Self {
            conservation,
            #[cfg(debug_assertions)]
            malformed_verbatim_count: 0,
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn consume(&mut self, claim: SourceClaim<'tree>) -> Result<(), ConservationError> {
        match claim {
            SourceClaim::Identity(identity) => self.conservation.claim(identity)?,
            SourceClaim::MalformedVerbatim { claim, .. } => {
                self.conservation.claim_source_range(claim)?;
                #[cfg(debug_assertions)]
                {
                    self.malformed_verbatim_count += 1;
                }
            }
            SourceClaim::Replaced { source, .. } => self.conservation.claim_token(source)?,
            SourceClaim::Removed { source, .. } => self.conservation.claim(source)?,
            SourceClaim::Synthesized { anchor, .. } | SourceClaim::Reordered { anchor, .. } => {
                self.conservation.validate_token(anchor)?;
            }
            SourceClaim::FormatterIgnore { range } => {
                self.conservation.claim_source_range(range)?;
            }
        }
        Ok(())
    }

    #[cfg_attr(not(debug_assertions), allow(dead_code))]
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
