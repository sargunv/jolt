//! Java field/list recovery resolution.
//!
//! Resolution itself is shared in `jolt_fmt_ir::recovery`; this module binds it
//! to Java's lexical-safety policy.

use jolt_fmt_ir::{Doc, DocBuilder, FormatDelimiter, FormatField, FormatListPart};
use jolt_java_syntax::{
    JavaMissingSyntax, JavaSyntaxField, JavaSyntaxListPart, JavaSyntaxToken, JavaSyntaxView,
};

use super::lexical_safety::JavaLexicalSafety;

pub(crate) type JavaFormatField<'source, T> = FormatField<'source, T>;

pub(crate) type JavaFormatListPart<'source, T> =
    FormatListPart<'source, T, JavaSyntaxToken<'source>>;

pub(crate) type JavaFormatDelimiter<'source> = FormatDelimiter<'source, JavaSyntaxToken<'source>>;

/// Formats one syntax-owned malformed boundary and claims its exact source.
pub(crate) fn format_malformed<'source>(
    malformed: &impl JavaSyntaxView<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    jolt_fmt_ir::format_malformed_core(
        doc,
        malformed.malformed_verbatim_core(),
        &mut JavaLexicalSafety,
        "malformed syntax did not own a verbatim core",
    )
}

pub(crate) fn format_missing<'source>(
    missing: &JavaMissingSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    jolt_fmt_ir::format_missing(doc, missing)
}

pub(crate) fn resolve_required_delimiter<'source>(
    field: JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatDelimiter<'source> {
    jolt_fmt_ir::resolve_required_delimiter(field, doc, &mut JavaLexicalSafety)
}

/// The token a field holds, for rules that only need to read its trivia and
/// leave recovery to whoever formats the field.
pub(crate) fn present_token<'source>(
    field: JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
) -> Option<JavaSyntaxToken<'source>> {
    match field {
        JavaSyntaxField::Present(token) => Some(token),
        JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => None,
    }
}

/// Whether a field's recovery renders no source of its own: a missing slot,
/// or a malformed node that owns no tokens. The recovery document still
/// carries its claims; this only reports that it emits no visible text.
pub(crate) fn field_is_claim_only<T>(field: &JavaSyntaxField<'_, T>) -> bool {
    match field {
        JavaSyntaxField::Present(_) => false,
        JavaSyntaxField::Missing(_) => true,
        JavaSyntaxField::Malformed(malformed) => malformed.syntax().first_token().is_none(),
    }
}

pub(crate) fn resolve_list_part<'source, T>(
    part: JavaSyntaxListPart<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatListPart<'source, T> {
    jolt_fmt_ir::resolve_list_part(part, doc, &mut JavaLexicalSafety)
}

pub(crate) fn resolve_required_field<'source, T>(
    field: JavaSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatField<'source, T> {
    jolt_fmt_ir::resolve_required_field(field, doc, &mut JavaLexicalSafety)
}

pub(crate) fn resolve_optional_field<'source, T>(
    field: JavaSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatField<'source, Option<T>> {
    jolt_fmt_ir::resolve_optional_field(field, doc, &mut JavaLexicalSafety)
}

pub(crate) fn format_required_field<'source, T>(
    field: JavaSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    jolt_fmt_ir::format_required_field(resolve_required_field(field, doc), doc, structured)
}

pub(crate) fn format_optional_field<'source, T>(
    field: JavaSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    jolt_fmt_ir::format_optional_field(resolve_optional_field(field, doc), doc, structured)
}
