//! Field/list recovery resolution for the Kotlin formatter.
//!
//! Shared present/malformed field results and malformed fragment assembly live
//! in `jolt_fmt_ir::recovery`. This module owns Kotlin field/list resolution
//! against typed CST enums, including invisible list parts.

use jolt_fmt_ir::{Doc, DocBuilder, FormatField, assemble_malformed_fragment};
use jolt_kotlin_syntax::{
    KotlinMissingSyntax, KotlinSyntaxField, KotlinSyntaxListPart, KotlinSyntaxToken,
    KotlinSyntaxView,
};

use super::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_comment,
    format_leading_comment_list, format_token,
};
use super::lexical_safety::KotlinLexicalSafety;

pub(crate) type KotlinFormatField<'source, T> = FormatField<'source, T>;

/// Formats one syntax-owned malformed boundary and claims its exact source.
pub(crate) fn format_malformed<'source>(
    malformed: &impl KotlinSyntaxView<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(core) = malformed.malformed_verbatim_core() else {
        doc.block_on_invariant("malformed Kotlin syntax did not own a verbatim core");
        return Doc::nil();
    };
    let (leading, trailing, has_leading_comments, has_trailing_comments) =
        malformed_boundary_comments(&core, doc);
    let mut safety = KotlinLexicalSafety;
    assemble_malformed_fragment(
        doc,
        &core,
        &mut safety,
        leading,
        trailing,
        has_leading_comments,
        has_trailing_comments,
    )
}

pub(crate) enum KotlinFormatListPart<'source, T> {
    Item(T),
    Separator(KotlinSyntaxToken<'source>),
    Malformed(Doc<'source>),
    Invisible(Doc<'source>),
}

/// A delimiter slot resolved without losing its exact source position.
#[derive(Clone, Copy)]
pub(crate) enum KotlinFormatDelimiter<'source> {
    Source(KotlinSyntaxToken<'source>),
    Recovery(Doc<'source>),
}

impl<'source> KotlinFormatDelimiter<'source> {
    pub(crate) fn source(&self) -> Option<&KotlinSyntaxToken<'source>> {
        match self {
            Self::Source(token) => Some(token),
            Self::Recovery(_) => None,
        }
    }

    const fn recovery(&self) -> Doc<'source> {
        match self {
            Self::Source(_) => Doc::nil(),
            Self::Recovery(recovery) => *recovery,
        }
    }
}

pub(crate) fn format_delimiter_with_preserved_trailing<'source>(
    doc: &mut DocBuilder<'source>,
    delimiter: KotlinFormatDelimiter<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match delimiter {
        KotlinFormatDelimiter::Source(token) => {
            format_token(doc, &token, leading, TrailingTrivia::Preserve)
        }
        KotlinFormatDelimiter::Recovery(recovery) => recovery,
    }
}

pub(crate) fn join_delimited_recovery<'source>(
    doc: &mut DocBuilder<'source>,
    open: &KotlinFormatDelimiter<'source>,
    contents: Doc<'source>,
    close: &KotlinFormatDelimiter<'source>,
) -> Doc<'source> {
    doc.concat([open.recovery(), contents, close.recovery()])
}

pub(crate) fn resolve_required_delimiter<'source>(
    field: KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatDelimiter<'source> {
    match resolve_required_field(field, doc) {
        FormatField::Present(token) => KotlinFormatDelimiter::Source(token),
        FormatField::Malformed(recovery) => KotlinFormatDelimiter::Recovery(recovery),
    }
}

pub(crate) fn resolve_list_part<'source, T>(
    part: KotlinSyntaxListPart<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatListPart<'source, T> {
    match part {
        KotlinSyntaxListPart::Item(item) => KotlinFormatListPart::Item(item),
        KotlinSyntaxListPart::Separator(separator) => KotlinFormatListPart::Separator(separator),
        KotlinSyntaxListPart::Missing(missing) => {
            KotlinFormatListPart::Invisible(format_missing(&missing, doc))
        }
        KotlinSyntaxListPart::Malformed(malformed) => {
            let recovery = format_malformed(&malformed, doc);
            if malformed.first_token().is_some() {
                KotlinFormatListPart::Malformed(recovery)
            } else {
                KotlinFormatListPart::Invisible(recovery)
            }
        }
    }
}

// On WASM, these generic field resolvers are deliberate codegen boundaries.
// They run for present as well as malformed syntax; `inline(never)` is not a
// cold-path hint. Native inlining remains optimizer-controlled. Re-measure
// formatter throughput and optimized WASM size before changing this policy.
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub(crate) fn resolve_required_field<'source, T>(
    field: KotlinSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatField<'source, T> {
    match field {
        KotlinSyntaxField::Present(value) => FormatField::Present(value),
        KotlinSyntaxField::Malformed(malformed) => {
            FormatField::Malformed(format_malformed(&malformed, doc))
        }
        KotlinSyntaxField::Missing(missing) => {
            FormatField::Malformed(format_missing(&missing, doc))
        }
    }
}

#[cfg_attr(target_arch = "wasm32", inline(never))]
pub(crate) fn resolve_optional_field<'source, T>(
    field: KotlinSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatField<'source, Option<T>> {
    match field {
        KotlinSyntaxField::Present(value) => FormatField::Present(Some(value)),
        KotlinSyntaxField::Missing(_) => FormatField::Present(None),
        KotlinSyntaxField::Malformed(malformed) => {
            FormatField::Malformed(format_malformed(&malformed, doc))
        }
    }
}

pub(crate) fn format_required_field<'source, T>(
    field: KotlinSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    jolt_fmt_ir::format_required_field(resolve_required_field(field, doc), doc, structured)
}

pub(crate) fn format_optional_field<'source, T>(
    field: KotlinSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    jolt_fmt_ir::format_optional_field(resolve_optional_field(field, doc), doc, structured)
}

pub(crate) fn format_missing<'source>(
    missing: &KotlinMissingSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if missing.verbatim_core().is_err() {
        doc.block_on_invariant("missing Kotlin role did not own an empty verbatim core");
    }
    Doc::nil()
}

fn malformed_boundary_comments<'source>(
    core: &jolt_kotlin_syntax::KotlinSyntaxVerbatimCore<'source>,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, Doc<'source>, bool, bool) {
    let leading_comments = core
        .first_token()
        .into_iter()
        .flat_map(|token| token.leading_comments())
        .filter(|comment| !core.contains(comment.text_range()));
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
    (
        leading,
        trailing,
        has_leading_comments,
        has_trailing_comments,
    )
}
