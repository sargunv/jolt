//! Shared recovery field scaffolding for language formatters.
//!
//! Language crates own resolution against their typed CST enums. This module
//! holds the shared resolved field/list shapes and malformed-fragment assembly.

use jolt_syntax::{
    Language, MalformedSyntax, MissingSyntax, SyntaxField, SyntaxListPart, SyntaxToken,
    SyntaxVerbatimCore,
};

use crate::comments::{comment_forces_line, format_comment, format_leading_comment_list};

use crate::source_fragment::LexicalSafety;
use crate::{Doc, DocBuilder};

/// Structured field result after recovery has been claimed as a document.
#[derive(Clone, Copy)]
pub enum FormatField<'source, T> {
    Present(T),
    Malformed(Doc<'source>),
}

/// A resolved delimiter slot, preserving either its source token or recovery
/// document.
#[derive(Clone, Copy)]
pub enum FormatDelimiter<'source, Token> {
    Source(Token),
    Recovery(LayoutDoc<'source>),
}

impl<'source, Token> FormatDelimiter<'source, Token> {
    #[must_use]
    pub const fn source(&self) -> Option<&Token> {
        match self {
            Self::Source(token) => Some(token),
            Self::Recovery(_) => None,
        }
    }

    #[must_use]
    pub const fn recovery(&self) -> Doc<'source> {
        match self {
            Self::Source(_) => Doc::nil(),
            Self::Recovery(recovery) => recovery.doc(),
        }
    }

    #[must_use]
    pub const fn is_visible(&self) -> bool {
        match self {
            Self::Source(_) => true,
            Self::Recovery(recovery) => recovery.is_visible(),
        }
    }
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

/// Formats one syntax-owned malformed boundary, claiming its exact source.
///
/// Comments that sit outside the verbatim core are relocated around it, so the
/// core claims only the source it actually owns.
pub fn format_malformed_core<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    core: Option<SyntaxVerbatimCore<'source, L>>,
    safety: &mut impl LexicalSafety<L>,
    missing_core_invariant: &'static str,
) -> Doc<'source> {
    let Some(core) = core else {
        doc.block_on_invariant(missing_core_invariant);
        return Doc::nil();
    };

    // An enclosing construct that relocated the first token's leading trivia
    // owns those comments; the core emits only the source it still owns.
    let leading_relocated = core
        .first_token()
        .is_some_and(|token| doc.relocates_leading_trivia(&token));
    let leading_comments = core
        .first_token()
        .into_iter()
        .flat_map(|token| token.leading_comments())
        .filter(|comment| !leading_relocated && !core.contains(comment.text_range()));
    let has_leading_comments = leading_comments.clone().next().is_some();
    let leading = format_leading_comment_list(doc, leading_comments);

    let trailing_comments = core
        .last_token()
        .into_iter()
        .flat_map(|token| token.trailing_comments())
        .filter(|comment| !core.contains(comment.text_range()));
    let has_trailing_comments = trailing_comments.clone().next().is_some();
    let trailing = doc.concat_list(|comments| {
        for comment in trailing_comments {
            let space = comments.space();
            comments.push(space);
            let forces_line = comment_forces_line(&comment);
            let comment = format_comment(comments, &comment);
            comments.push(comment);
            if forces_line {
                let line = comments.hard_line();
                comments.push(line);
            }
        }
    });

    assemble_malformed_fragment(
        doc,
        &core,
        safety,
        leading,
        trailing,
        has_leading_comments,
        has_trailing_comments,
    )
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

/// Formats one syntax-owned malformed node, claiming its exact source.
pub fn format_malformed<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    malformed: &MalformedSyntax<'source, L>,
    safety: &mut impl LexicalSafety<L>,
) -> Doc<'source> {
    format_malformed_core(
        doc,
        malformed.syntax().malformed_verbatim_core(),
        safety,
        "malformed syntax did not own a verbatim core",
    )
}

/// Claims the zero-width boundary a missing slot represents.
pub fn format_missing<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    missing: &MissingSyntax<'source, L>,
) -> Doc<'source> {
    if missing.verbatim_core().is_err() {
        doc.block_on_invariant("missing role did not own an empty verbatim core");
    }
    Doc::nil()
}

/// Resolves a required delimiter slot, keeping its source token or recovery doc.
pub fn resolve_required_delimiter<'source, L: Language>(
    field: SyntaxField<'source, L, SyntaxToken<'source, L>>,
    doc: &mut DocBuilder<'source>,
    safety: &mut impl LexicalSafety<L>,
) -> FormatDelimiter<'source, SyntaxToken<'source, L>> {
    match field {
        SyntaxField::Present(token) => FormatDelimiter::Source(token),
        SyntaxField::Missing(missing) => {
            FormatDelimiter::Recovery(LayoutDoc::ClaimOnly(format_missing(doc, &missing)))
        }
        SyntaxField::Malformed(malformed) => {
            let recovery = format_malformed(doc, &malformed, safety);
            FormatDelimiter::Recovery(LayoutDoc::from_visibility(
                recovery,
                malformed.syntax().first_token().is_some(),
            ))
        }
    }
}

/// Resolves one physical syntax-list part.
pub fn resolve_list_part<'source, L: Language, T>(
    part: SyntaxListPart<'source, L, T>,
    doc: &mut DocBuilder<'source>,
    safety: &mut impl LexicalSafety<L>,
) -> FormatListPart<'source, T, SyntaxToken<'source, L>> {
    match part {
        SyntaxListPart::Item(item) => FormatListPart::Item(item),
        SyntaxListPart::Separator(separator) => FormatListPart::Separator(separator),
        SyntaxListPart::Missing(missing) => {
            FormatListPart::Recovery(LayoutDoc::ClaimOnly(format_missing(doc, &missing)))
        }
        SyntaxListPart::Malformed(malformed) => {
            let recovery = format_malformed(doc, &malformed, safety);
            FormatListPart::Recovery(LayoutDoc::from_visibility(
                recovery,
                malformed.syntax().first_token().is_some(),
            ))
        }
    }
}

// On WASM, these generic field resolvers are deliberate codegen boundaries.
// They run for present as well as malformed syntax; `inline(never)` is not a
// cold-path hint. Native inlining remains optimizer-controlled. Re-measure
// formatter throughput and optimized WASM size before changing this policy.
/// Resolves one generated field without letting missing or malformed syntax
/// leak into a structured layout rule.
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn resolve_required_field<'source, L: Language, T>(
    field: SyntaxField<'source, L, T>,
    doc: &mut DocBuilder<'source>,
    safety: &mut impl LexicalSafety<L>,
) -> FormatField<'source, T> {
    match field {
        SyntaxField::Present(value) => FormatField::Present(value),
        SyntaxField::Malformed(malformed) => {
            FormatField::Malformed(format_malformed(doc, &malformed, safety))
        }
        SyntaxField::Missing(missing) => FormatField::Malformed(format_missing(doc, &missing)),
    }
}

/// Resolves an optional generated field; its empty slot is ordinary absence.
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub fn resolve_optional_field<'source, L: Language, T>(
    field: SyntaxField<'source, L, T>,
    doc: &mut DocBuilder<'source>,
    safety: &mut impl LexicalSafety<L>,
) -> FormatField<'source, Option<T>> {
    match field {
        SyntaxField::Present(value) => FormatField::Present(Some(value)),
        SyntaxField::Missing(_) => FormatField::Present(None),
        SyntaxField::Malformed(malformed) => {
            FormatField::Malformed(format_malformed(doc, &malformed, safety))
        }
    }
}
