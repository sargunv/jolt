use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{CallableReferenceExpression, Expression, TypeReference};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_required_field, resolve_list_part,
    resolve_optional_field,
};
use crate::rules::types::format_type_argument_list;

use super::format_expression_with_leading;

pub(super) fn format_callable_reference_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CallableReferenceExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let (receiver, has_receiver) = match resolve_optional_field(expression.receiver(), doc) {
        KotlinFormatField::Present(Some(receiver)) => {
            if let Some(receiver) = receiver.cast_family::<Expression<'source>>() {
                (
                    format_expression_with_leading(doc, &receiver, leading),
                    true,
                )
            } else if let Some(receiver) = receiver.cast_node::<TypeReference<'source>>() {
                (
                    crate::rules::types::format_type_reference(doc, &receiver),
                    true,
                )
            } else {
                doc.block_on_invariant("callable-reference receiver had an unknown declared role");
                (Doc::nil(), true)
            }
        }
        KotlinFormatField::Present(None) => (Doc::nil(), false),
        KotlinFormatField::Malformed(recovery) => (recovery, true),
    };
    let separator = format_required_field(expression.separator(), doc, |separator, doc| {
        format_token(
            doc,
            &separator,
            if has_receiver {
                LeadingTrivia::Preserve
            } else {
                leading
            },
            TrailingTrivia::Preserve,
        )
    });
    let target = format_required_field(expression.target(), doc, |target, doc| {
        format_token(
            doc,
            &target,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let arguments = format_required_field(expression.type_arguments(), doc, |arguments, doc| {
        doc.concat_list(|docs| {
            for part in arguments.parts() {
                let part = match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(arguments) => {
                        format_type_argument_list(docs, &arguments)
                    }
                    KotlinFormatListPart::Separator(separator) => format_token(
                        docs,
                        &separator,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::Preserve,
                    ),
                    KotlinFormatListPart::Malformed(recovery) => recovery,
                };
                docs.push(part);
            }
        })
    });
    let contents = doc.concat([receiver, separator, target, arguments]);
    doc.group(contents)
}
