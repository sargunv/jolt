use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{CallableReferenceExpression, KotlinSyntaxToken};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::rules::types::format_type_argument_list;

use super::format_expression_with_leading;

pub(super) fn format_callable_reference_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CallableReferenceExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(separator) = expression.separator_token() else {
        return format_callable_reference_parts(doc, expression, leading);
    };
    let Some(target) = expression.target_token() else {
        return format_callable_reference_parts(doc, expression, leading);
    };

    let receiver = if let Some(receiver) = expression.receiver() {
        format_expression_with_leading(doc, &receiver, leading)
    } else {
        doc.nil()
    };
    let separator = format_token(
        doc,
        &separator,
        if expression.receiver().is_some() {
            LeadingTrivia::Preserve
        } else {
            leading
        },
        TrailingTrivia::Preserve,
    );
    let before_target = format_type_arguments_before_target(doc, expression, &target);
    let after_target = format_type_arguments_after_target(doc, expression, &target);
    let target = format_token(
        doc,
        &target,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    let contents = doc.concat([receiver, separator, before_target, target, after_target]);
    doc.group(contents)
}

fn format_callable_reference_parts<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CallableReferenceExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let receiver = if let Some(receiver) = expression.receiver() {
        format_expression_with_leading(doc, &receiver, leading)
    } else {
        doc.nil()
    };
    let separator = if let Some(separator) = expression.separator_token() {
        format_token(
            doc,
            &separator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let target = if let Some(target) = expression.target_token() {
        format_token(
            doc,
            &target,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    let arguments = expression
        .type_argument_lists()
        .map(|arguments| format_type_argument_list(doc, &arguments))
        .collect::<Vec<_>>();
    let arguments = doc.concat(arguments);
    let contents = doc.concat([receiver, separator, arguments, target]);
    doc.group(contents)
}

fn format_type_arguments_before_target<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CallableReferenceExpression<'source>,
    target: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let arguments = expression
        .type_argument_lists()
        .filter(|arguments| arguments.text_range().end() <= target.token_text_range().start())
        .map(|arguments| format_type_argument_list(doc, &arguments))
        .collect::<Vec<_>>();
    doc.concat(arguments)
}

fn format_type_arguments_after_target<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &CallableReferenceExpression<'source>,
    target: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let arguments = expression
        .type_argument_lists()
        .filter(|arguments| arguments.text_range().start() >= target.token_text_range().end())
        .map(|arguments| format_type_argument_list(doc, &arguments))
        .collect::<Vec<_>>();
    doc.concat(arguments)
}
