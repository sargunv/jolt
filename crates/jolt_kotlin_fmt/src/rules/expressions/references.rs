use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{CallableReferenceExpression, CallableReferenceReceiverSyntax};

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
            let receiver = format_required_field(receiver.receiver(), doc, |receiver, doc| {
                match receiver.classify() {
                    Ok(CallableReferenceReceiverSyntax::Expression(receiver)) => {
                        format_expression_with_leading(doc, &receiver, leading)
                    }
                    Ok(CallableReferenceReceiverSyntax::TypeReference(receiver)) => {
                        crate::rules::types::format_type_reference(doc, &receiver)
                    }
                    Err(error) => {
                        doc.block_on_invariant(error.to_string());
                        Doc::nil()
                    }
                }
            });
            (receiver, true)
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
        format_required_field(target.target(), doc, |target, doc| {
            format_token(
                doc,
                &target,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        })
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
