//! Shared recovery field scaffolding for language formatters.
//!
//! Language crates own field/list resolution against their typed CST enums and
//! layout-specific recovery states such as Kotlin's invisible list parts. This
//! module holds the pieces that are the same specification: present-or-malformed
//! field results, and assembling a malformed verbatim fragment once boundary
//! comments and neighbor tokens are known.

use jolt_syntax::{Language, SyntaxToken, SyntaxVerbatimCore};

use crate::source_fragment::LexicalSafety;
use crate::{Doc, DocBuilder};

/// Structured field result after recovery has been claimed as a document.
#[derive(Clone, Copy)]
pub enum FormatField<'source, T> {
    Present(T),
    Malformed(Doc<'source>),
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
