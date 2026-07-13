use std::borrow::Cow;

use jolt_syntax::{
    ConservationError, Language, SourceIdentity, SourceTokenId, SyntaxConservationTracker,
    SyntaxToken,
};

use crate::{
    Doc,
    width::{TextWidth, literal_text_metrics},
};

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

/// A closed, semantics-preserving token normalization already used by Jolt.
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

    pub(crate) const fn lexical_kind(self) -> LexicalAtomKind {
        match self {
            Self::ImportKeyword | Self::ImportAliasKeyword => LexicalAtomKind::Identifier,
            _ => LexicalAtomKind::Punctuation,
        }
    }
}

/// A closed reason for consuming represented syntax without emitting it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemovalReason {
    DuplicateImport,
    RedundantSeparator,
}

/// Syntax-authorized permission to replace one represented source token.
///
/// The syntax layer will construct these claims once generated grammar roles
/// can prove that the normalization is valid at the owning slot. Formatter
/// rules cannot manufacture a claim from an arbitrary token identity.
pub struct ReplacementClaim<'tree> {
    source: SourceTokenId<'tree>,
    token: NormalizedToken,
}

impl<'tree> ReplacementClaim<'tree> {
    pub(crate) const fn into_parts(self) -> (SourceTokenId<'tree>, NormalizedToken) {
        (self.source, self.token)
    }

    #[cfg(test)]
    pub(crate) const fn for_test(source: SourceTokenId<'tree>, token: NormalizedToken) -> Self {
        Self { source, token }
    }
}

/// Syntax-authorized permission to consume represented syntax without output.
pub struct RemovalClaim<'tree> {
    source: SourceIdentity<'tree>,
    reason: RemovalReason,
}

impl<'tree> RemovalClaim<'tree> {
    pub(crate) const fn into_parts(self) -> (SourceIdentity<'tree>, RemovalReason) {
        (self.source, self.reason)
    }

    #[cfg(test)]
    pub(crate) const fn for_test(source: SourceIdentity<'tree>, reason: RemovalReason) -> Self {
        Self { source, reason }
    }
}

/// Syntax-authorized permission to synthesize one normalization token.
pub struct SynthesisClaim<'tree> {
    anchor: SourceTokenId<'tree>,
    token: NormalizedToken,
}

impl<'tree> SynthesisClaim<'tree> {
    pub(crate) const fn into_parts(self) -> (SourceTokenId<'tree>, NormalizedToken) {
        (self.anchor, self.token)
    }

    #[cfg(test)]
    pub(crate) const fn for_test(anchor: SourceTokenId<'tree>, token: NormalizedToken) -> Self {
        Self { anchor, token }
    }
}

/// The exceptional source operation represented by a fragment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceFragmentKind<'tree> {
    MalformedVerbatim,
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
}

/// Debug/test evidence emitted only for the exceptional fragments the renderer
/// actually visits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderedSourceFragment<'tree> {
    pub kind: SourceFragmentKind<'tree>,
}

/// Render-time conservation proof and exceptional-fragment ledger.
///
/// The syntax tracker compiles to zero state in optimized builds. The fragment
/// ledger likewise exists only with debug assertions, so ordinary release
/// formatting adds no tracker or comment-map allocation.
pub struct RenderProof<'tree> {
    conservation: SyntaxConservationTracker<'tree>,
    #[cfg(debug_assertions)]
    rendered: Vec<RenderedSourceFragment<'tree>>,
}

impl<'tree> RenderProof<'tree> {
    #[must_use]
    pub fn new(conservation: SyntaxConservationTracker<'tree>) -> Self {
        Self {
            conservation,
            #[cfg(debug_assertions)]
            rendered: Vec::new(),
        }
    }

    pub(crate) fn render_fragment<'source>(
        &mut self,
        fragment: &SourceFragment<'source, 'tree>,
        claims: &[SourceIdentity<'tree>],
    ) -> Result<(), ConservationError> {
        #[cfg(not(debug_assertions))]
        let _ = fragment;
        #[cfg(debug_assertions)]
        if let Some(SourceFragmentKind::Synthesized { anchor, .. }) = fragment.provenance {
            self.conservation.validate_token(anchor)?;
        }
        for identity in claims {
            self.conservation.claim(*identity)?;
        }
        #[cfg(debug_assertions)]
        if let Some(kind) = fragment.provenance {
            self.rendered.push(RenderedSourceFragment { kind });
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<CompletedRenderProof<'tree>, ConservationError> {
        self.conservation.finish()?;
        Ok(CompletedRenderProof {
            #[cfg(debug_assertions)]
            rendered: self.rendered,
            #[cfg(not(debug_assertions))]
            marker: std::marker::PhantomData,
        })
    }
}

/// A completed source-conservation proof returned by tracked rendering.
pub struct CompletedRenderProof<'tree> {
    #[cfg(debug_assertions)]
    rendered: Vec<RenderedSourceFragment<'tree>>,
    #[cfg(not(debug_assertions))]
    marker: std::marker::PhantomData<&'tree ()>,
}

impl<'tree> CompletedRenderProof<'tree> {
    #[must_use]
    pub fn rendered_fragments(&self) -> &[RenderedSourceFragment<'tree>] {
        #[cfg(debug_assertions)]
        {
            &self.rendered
        }
        #[cfg(not(debug_assertions))]
        {
            &[]
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceFragment<'source, 'tree> {
    pub(crate) text: Cow<'source, str>,
    #[cfg(debug_assertions)]
    pub(crate) provenance: Option<SourceFragmentKind<'tree>>,
    #[cfg(debug_assertions)]
    pub(crate) claims_start: u32,
    #[cfg(debug_assertions)]
    pub(crate) claims_len: u32,
    #[cfg(not(debug_assertions))]
    marker: std::marker::PhantomData<&'tree ()>,
    final_width: TextWidth,
    line_count: usize,
}

impl<'source, 'tree> SourceFragment<'source, 'tree> {
    pub(crate) fn new(
        text: Cow<'source, str>,
        provenance: Option<SourceFragmentKind<'tree>>,
        #[cfg(debug_assertions)] claims_start: u32,
        #[cfg(debug_assertions)] claims_len: u32,
    ) -> Self {
        let metrics = literal_text_metrics(&text);
        #[cfg(not(debug_assertions))]
        let _ = provenance;
        Self {
            text,
            #[cfg(debug_assertions)]
            provenance,
            #[cfg(debug_assertions)]
            claims_start,
            #[cfg(debug_assertions)]
            claims_len,
            #[cfg(not(debug_assertions))]
            marker: std::marker::PhantomData,
            final_width: metrics.final_width,
            line_count: metrics.line_count,
        }
    }

    pub(crate) const fn final_width(&self) -> TextWidth {
        self.final_width
    }

    pub(crate) const fn is_multiline(&self) -> bool {
        self.line_count > 1
    }
}
