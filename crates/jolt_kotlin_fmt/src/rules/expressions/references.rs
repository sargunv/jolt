use jolt_fmt_ir::{Doc, concat, group};
use jolt_kotlin_syntax::{CallableReferenceExpression, KotlinSyntaxToken};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::rules::types::format_type_argument_list;

use super::format_expression_with_leading;

pub(super) fn format_callable_reference_expression<'source>(
    expression: &CallableReferenceExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(separator) = expression.separator_token() else {
        return format_callable_reference_parts(expression, leading);
    };
    let Some(target) = expression.target_token() else {
        return format_callable_reference_parts(expression, leading);
    };

    group(concat([
        expression
            .receiver()
            .map_or_else(jolt_fmt_ir::nil, |receiver| {
                format_expression_with_leading(&receiver, leading)
            }),
        format_token(
            &separator,
            if expression.receiver().is_some() {
                LeadingTrivia::Preserve
            } else {
                leading
            },
            TrailingTrivia::Preserve,
        ),
        format_type_arguments_before_target(expression, &target),
        format_token(&target, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        format_type_arguments_after_target(expression, &target),
    ]))
}

fn format_callable_reference_parts<'source>(
    expression: &CallableReferenceExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    group(concat([
        expression
            .receiver()
            .map_or_else(jolt_fmt_ir::nil, |receiver| {
                format_expression_with_leading(&receiver, leading)
            }),
        expression
            .separator_token()
            .map_or_else(jolt_fmt_ir::nil, |separator| {
                format_token(
                    &separator,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                )
            }),
        expression
            .target_token()
            .map_or_else(jolt_fmt_ir::nil, |target| {
                format_token(&target, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
            }),
    ]))
}

fn format_type_arguments_before_target<'source>(
    expression: &CallableReferenceExpression<'source>,
    target: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    concat(
        expression
            .type_argument_lists()
            .filter(|arguments| arguments.text_range().end() <= target.token_text_range().start())
            .map(|arguments| format_type_argument_list(&arguments)),
    )
}

fn format_type_arguments_after_target<'source>(
    expression: &CallableReferenceExpression<'source>,
    target: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    concat(
        expression
            .type_argument_lists()
            .filter(|arguments| arguments.text_range().start() >= target.token_text_range().end())
            .map(|arguments| format_type_argument_list(&arguments)),
    )
}
