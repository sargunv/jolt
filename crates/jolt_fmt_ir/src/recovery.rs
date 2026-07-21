//! Shared recovery field scaffolding for language formatters.
//!
//! Language crates own field/list resolve against their typed CST enums and any
//! recovery-policy extras (for example Kotlin's invisible list parts). This
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

/// How empty malformed cores choose exceptional neighbors for lexical safety.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MalformedBoundaryPolicy {
    /// Skip neighbor resolution when the core has no tokens **or** an empty
    /// text range (Java).
    RequireNonEmptyRange,
    /// Skip neighbor resolution only when the core has no tokens (Kotlin).
    TokensOnly,
}

/// Assembles leading comments + malformed verbatim + trailing comments with the
/// language-chosen empty-core boundary policy.
#[allow(clippy::too_many_arguments)]
pub fn assemble_malformed_fragment<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    core: &SyntaxVerbatimCore<'source, L>,
    policy: MalformedBoundaryPolicy,
    safety: &mut impl LexicalSafety<L>,
    leading: Doc<'source>,
    trailing: Doc<'source>,
    has_leading_comments: bool,
    has_trailing_comments: bool,
) -> Doc<'source> {
    let has_tokens = core.tokens().next().is_some();
    let use_neighbors = match policy {
        MalformedBoundaryPolicy::RequireNonEmptyRange => {
            let range = core.text_range();
            has_tokens && range.start() != range.end()
        }
        MalformedBoundaryPolicy::TokensOnly => has_tokens,
    };
    let (left, right): (
        Option<SyntaxToken<'source, L>>,
        Option<SyntaxToken<'source, L>>,
    ) = if use_neighbors {
        (
            (!has_leading_comments)
                .then(|| core.previous_token())
                .flatten(),
            (!has_trailing_comments)
                .then(|| core.next_token())
                .flatten(),
        )
    } else {
        (None, None)
    };
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
