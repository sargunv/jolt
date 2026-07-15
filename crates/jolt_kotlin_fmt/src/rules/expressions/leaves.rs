use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Expression, KotlinRoleElement, KotlinSyntaxToken, LiteralExpression, LongStringTemplateEntry,
    NameExpression, StringTemplateEntry, StringTemplateExpression, SuperExpression, ThisExpression,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::recovery::{
    KotlinFormatListPart, format_optional_field, format_required_field, resolve_list_part,
};
use crate::rules::types::format_type_argument_list;

use super::{format_expression, format_expression_with_leading};

pub(super) fn format_literal_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &LiteralExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_required_field(expression.literal(), doc, |token, doc| {
        format_token(doc, &token, leading, TrailingTrivia::Preserve)
    })
}

pub(super) fn format_name_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &NameExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_required_field(expression.name(), doc, |token, doc| {
        format_token(doc, &token, leading, TrailingTrivia::Preserve)
    })
}

pub(super) fn format_this_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &ThisExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let this = format_required_field(expression.this_token(), doc, |token, doc| {
        format_token(doc, &token, leading, TrailingTrivia::Preserve)
    });
    let label = format_label_suffix(doc, expression.at(), expression.label());
    doc.concat([this, label])
}

pub(super) fn format_super_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &SuperExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let super_token = format_required_field(expression.super_token(), doc, |token, doc| {
        format_token(doc, &token, leading, TrailingTrivia::Preserve)
    });
    let arguments = format_optional_field(expression.type_arguments(), doc, |arguments, doc| {
        format_type_argument_list(doc, &arguments)
    });
    let label = format_label_suffix(doc, expression.at(), expression.label());
    doc.concat([super_token, arguments, label])
}

fn format_label_suffix<'source>(
    doc: &mut DocBuilder<'source>,
    at: Result<
        jolt_kotlin_syntax::KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    label: Result<
        jolt_kotlin_syntax::KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
) -> Doc<'source> {
    let at = format_optional_field(at, doc, |at, doc| {
        format_token(doc, &at, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
    });
    let label = format_optional_field(label, doc, |label, doc| {
        format_token(
            doc,
            &label,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([at, label])
}

pub(super) fn format_string_template_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &StringTemplateExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let parts = format_required_field(expression.parts(), doc, |parts, doc| {
        let mut first = true;
        doc.concat_list(|docs| {
            for part in parts.parts() {
                let part_leading = if first {
                    leading
                } else {
                    LeadingTrivia::Preserve
                };
                let part = match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(entry) => {
                        format_string_template_entry(docs, &entry, part_leading)
                    }
                    KotlinFormatListPart::Separator(separator) => {
                        format_token(docs, &separator, part_leading, TrailingTrivia::Preserve)
                    }
                    KotlinFormatListPart::Malformed(recovery) => recovery,
                };
                docs.push(part);
                first = false;
            }
        })
    });
    let close = format_required_field(expression.close_quote(), doc, |close, doc| {
        format_token(
            doc,
            &close,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([parts, close])
}

fn format_string_template_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &StringTemplateEntry<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_required_field(
        entry.content(),
        doc,
        |content: KotlinRoleElement<'source>, doc| {
            if let Some(token) = content.token() {
                return format_token(doc, &token, leading, TrailingTrivia::Preserve);
            }
            if let Some(expression) = content.cast_family::<Expression<'source>>() {
                return format_expression_with_leading(doc, &expression, leading);
            }
            if let Some(long) = content.cast_node::<LongStringTemplateEntry<'source>>() {
                return format_long_string_template_entry(doc, &long, leading);
            }
            doc.block_on_invariant("string template entry had an unknown declared content role");
            Doc::nil()
        },
    )
}

fn format_long_string_template_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &LongStringTemplateEntry<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let open = format_required_field(entry.open(), doc, |open, doc| {
        format_token(doc, &open, leading, TrailingTrivia::Preserve)
    });
    let expression = format_required_field(entry.expression(), doc, |expression, doc| {
        format_expression(doc, &expression)
    });
    let close = format_required_field(entry.close(), doc, |close, doc| {
        format_token(
            doc,
            &close,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([open, expression, close])
}
