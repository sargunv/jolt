//! Shared recovery field scaffolding for language formatters.
//!
//! Language crates own resolution against their typed CST enums. This module
//! holds the shared resolved field/list shapes and malformed-fragment assembly.

use jolt_syntax::{Language, SyntaxToken, SyntaxVerbatimCore};

use crate::source_fragment::LexicalSafety;
use crate::{Doc, DocBuilder};

/// Structured field result after recovery has been claimed as a document.
#[derive(Clone, Copy)]
pub enum FormatField<'source, T> {
    Present(T),
    Malformed(Doc<'source>),
}

/// A formatted document's contribution to surrounding layout.
#[derive(Clone, Copy)]
pub enum LayoutDoc<'source> {
    Visible(Doc<'source>),
    ClaimOnly(Doc<'source>),
}

impl<'source> LayoutDoc<'source> {
    #[must_use]
    pub const fn from_visibility(doc: Doc<'source>, visible: bool) -> Self {
        if visible {
            Self::Visible(doc)
        } else {
            Self::ClaimOnly(doc)
        }
    }

    #[must_use]
    pub const fn doc(self) -> Doc<'source> {
        match self {
            Self::Visible(doc) | Self::ClaimOnly(doc) => doc,
        }
    }

    #[must_use]
    pub const fn is_visible(self) -> bool {
        matches!(self, Self::Visible(_))
    }
}

/// One resolved physical syntax-list part.
pub enum FormatListPart<'source, T, Separator> {
    Item(T),
    Separator(Separator),
    Recovery(LayoutDoc<'source>),
}

impl<T, Separator> FormatListPart<'_, T, Separator> {
    pub fn is_visible(
        &self,
        item_is_visible: impl FnOnce(&T) -> bool,
        separator_is_visible: impl FnOnce(&Separator) -> bool,
    ) -> bool {
        match self {
            Self::Item(item) => item_is_visible(item),
            Self::Separator(separator) => separator_is_visible(separator),
            Self::Recovery(recovery) => recovery.is_visible(),
        }
    }
}

/// Assembles leading comments + malformed verbatim + trailing comments with the
/// syntax-owned exceptional boundaries used for lexical safety.
#[allow(clippy::too_many_arguments)]
pub fn assemble_malformed_fragment<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    core: &SyntaxVerbatimCore<'source, L>,
    safety: &mut impl LexicalSafety<L>,
    leading: Doc<'source>,
    trailing: Doc<'source>,
    has_leading_comments: bool,
    has_trailing_comments: bool,
) -> Doc<'source> {
    let (left, right): (
        Option<SyntaxToken<'source, L>>,
        Option<SyntaxToken<'source, L>>,
    ) = (
        (!has_leading_comments)
            .then(|| core.previous_token())
            .flatten(),
        (!has_trailing_comments)
            .then(|| core.next_token())
            .flatten(),
    );
    let fragment = doc.malformed_verbatim_with_safety(core, safety);
    let fragment = doc.resolve_exceptional(fragment, left.as_ref(), right.as_ref(), safety);
    doc.concat([leading, fragment, trailing])
}

/// Applies a structured formatter to a resolved required field.
pub fn format_required_field<'source, T>(
    field: FormatField<'source, T>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    match field {
        FormatField::Present(value) => structured(value, doc),
        FormatField::Malformed(malformed) => malformed,
    }
}

/// Applies a structured formatter to a resolved optional field.
pub fn format_optional_field<'source, T>(
    field: FormatField<'source, Option<T>>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    match field {
        FormatField::Present(Some(value)) => structured(value, doc),
        FormatField::Present(None) => Doc::nil(),
        FormatField::Malformed(malformed) => malformed,
    }
}
