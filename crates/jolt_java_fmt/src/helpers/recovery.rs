//! Field/list recovery resolution for the Java formatter.
//!
//! Shared present/malformed field results and malformed fragment assembly live
//! in `jolt_fmt_ir::recovery`. This module owns Java field/list resolution
//! against typed CST enums.

use jolt_fmt_ir::{
    Doc, DocBuilder, FormatDelimiter, FormatField, FormatListPart, LayoutDoc, format_malformed_core,
};
use jolt_java_syntax::{
    JavaMissingSyntax, JavaSyntaxField, JavaSyntaxListPart, JavaSyntaxToken, JavaSyntaxView,
};

use super::lexical_safety::JavaLexicalSafety;

pub(crate) type JavaFormatField<'source, T> = FormatField<'source, T>;

/// Formats one syntax-owned malformed boundary and claims its exact source.
pub(crate) fn format_malformed<'source>(
    malformed: &impl JavaSyntaxView<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_malformed_core(
        doc,
        malformed.malformed_verbatim_core(),
        &mut JavaLexicalSafety,
        "malformed Java syntax did not own a verbatim core",
    )
}

pub(crate) type JavaFormatListPart<'source, T> =
    FormatListPart<'source, T, JavaSyntaxToken<'source>>;

pub(crate) type JavaFormatDelimiter<'source> = FormatDelimiter<'source, JavaSyntaxToken<'source>>;

pub(crate) fn resolve_required_delimiter<'source>(
    field: JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatDelimiter<'source> {
    match field {
        JavaSyntaxField::Present(token) => JavaFormatDelimiter::Source(token),
        JavaSyntaxField::Missing(missing) => {
            JavaFormatDelimiter::Recovery(LayoutDoc::ClaimOnly(format_missing(&missing, doc)))
        }
        JavaSyntaxField::Malformed(malformed) => {
            let recovery = format_malformed(&malformed, doc);
            JavaFormatDelimiter::Recovery(LayoutDoc::from_visibility(
                recovery,
                malformed.first_token().is_some(),
            ))
        }
    }
}

pub(crate) fn resolve_list_part<'source, T>(
    part: JavaSyntaxListPart<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatListPart<'source, T> {
    match part {
        JavaSyntaxListPart::Item(item) => JavaFormatListPart::Item(item),
        JavaSyntaxListPart::Separator(separator) => JavaFormatListPart::Separator(separator),
        JavaSyntaxListPart::Missing(missing) => {
            JavaFormatListPart::Recovery(LayoutDoc::ClaimOnly(format_missing(&missing, doc)))
        }
        JavaSyntaxListPart::Malformed(malformed) => {
            let recovery = format_malformed(&malformed, doc);
            JavaFormatListPart::Recovery(LayoutDoc::from_visibility(
                recovery,
                malformed.first_token().is_some(),
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
pub(crate) fn resolve_required_field<'source, T>(
    field: JavaSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatField<'source, T> {
    match field {
        JavaSyntaxField::Present(value) => FormatField::Present(value),
        JavaSyntaxField::Malformed(malformed) => {
            FormatField::Malformed(format_malformed(&malformed, doc))
        }
        JavaSyntaxField::Missing(missing) => FormatField::Malformed(format_missing(&missing, doc)),
    }
}

/// Resolves an optional generated field; its empty slot is ordinary absence.
#[cfg_attr(target_arch = "wasm32", inline(never))]
pub(crate) fn resolve_optional_field<'source, T>(
    field: JavaSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatField<'source, Option<T>> {
    match field {
        JavaSyntaxField::Present(value) => FormatField::Present(Some(value)),
        JavaSyntaxField::Missing(_) => FormatField::Present(None),
        JavaSyntaxField::Malformed(malformed) => {
            FormatField::Malformed(format_malformed(&malformed, doc))
        }
    }
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

pub(crate) fn format_missing<'source>(
    missing: &JavaMissingSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if missing.verbatim_core().is_err() {
        doc.block_on_invariant("missing Java role did not own an empty verbatim core");
    }
    Doc::nil()
}
