//! Kotlin field/list recovery resolution.
//!
//! Resolution itself is shared in `jolt_fmt_ir::recovery`; this module binds it
//! to Kotlin's lexical-safety policy and owns Kotlin's delimiter composition.

use jolt_fmt_ir::{Doc, DocBuilder, FormatDelimiter, FormatField, FormatListPart};
use jolt_kotlin_syntax::{
    KotlinMissingSyntax, KotlinSyntaxField, KotlinSyntaxListPart, KotlinSyntaxToken,
    KotlinSyntaxView,
};

use super::comments::{LeadingTrivia, TrailingTrivia, format_token};
use super::lexical_safety::KotlinLexicalSafety;

pub(crate) type KotlinFormatField<'source, T> = FormatField<'source, T>;

pub(crate) type KotlinFormatListPart<'source, T> =
    FormatListPart<'source, T, KotlinSyntaxToken<'source>>;

pub(crate) type KotlinFormatDelimiter<'source> =
    FormatDelimiter<'source, KotlinSyntaxToken<'source>>;

/// Formats one syntax-owned malformed boundary and claims its exact source.
pub(crate) fn format_malformed<'source>(
    malformed: &impl KotlinSyntaxView<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    jolt_fmt_ir::format_malformed_core(
        doc,
        malformed.malformed_verbatim_core(),
        &mut KotlinLexicalSafety,
        "malformed syntax did not own a verbatim core",
    )
}

pub(crate) fn format_missing<'source>(
    missing: &KotlinMissingSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    jolt_fmt_ir::format_missing(doc, missing)
}

pub(crate) fn resolve_required_delimiter<'source>(
    field: KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatDelimiter<'source> {
    jolt_fmt_ir::resolve_required_delimiter(field, doc, &mut KotlinLexicalSafety)
}

pub(crate) fn resolve_list_part<'source, T>(
    part: KotlinSyntaxListPart<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatListPart<'source, T> {
    jolt_fmt_ir::resolve_list_part(part, doc, &mut KotlinLexicalSafety)
}

pub(crate) fn resolve_required_field<'source, T>(
    field: KotlinSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatField<'source, T> {
    jolt_fmt_ir::resolve_required_field(field, doc, &mut KotlinLexicalSafety)
}

pub(crate) fn resolve_optional_field<'source, T>(
    field: KotlinSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatField<'source, Option<T>> {
    jolt_fmt_ir::resolve_optional_field(field, doc, &mut KotlinLexicalSafety)
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

pub(crate) fn format_delimiter_with_preserved_trailing<'source>(
    doc: &mut DocBuilder<'source>,
    delimiter: KotlinFormatDelimiter<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match delimiter {
        KotlinFormatDelimiter::Source(token) => {
            format_token(doc, &token, leading, TrailingTrivia::Preserve)
        }
        KotlinFormatDelimiter::Recovery(recovery) => recovery.doc(),
    }
}
