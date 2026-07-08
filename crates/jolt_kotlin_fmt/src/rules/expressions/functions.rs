use jolt_fmt_ir::{Doc, concat, space};
use jolt_kotlin_syntax::AnonymousFunctionExpression;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
};
use crate::rules::declarations::format_type_annotation;
use crate::rules::statements::format_block;
use crate::rules::types::format_type_reference;
use crate::rules::variables::format_value_parameter_list;

use super::format_expression;

pub(super) fn format_anonymous_function_expression<'source>(
    expression: &AnonymousFunctionExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(fun_token) = expression.fun_token() else {
        return format_token_sequence(expression.token_iter(), leading);
    };

    concat([
        format_token(
            &fun_token,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        format_anonymous_function_receiver(expression),
        expression
            .value_parameter_list()
            .map_or_else(jolt_fmt_ir::nil, |parameters| {
                format_value_parameter_list(&parameters)
            }),
        format_type_annotation(expression.colon(), expression.return_type()),
        format_anonymous_function_tail(expression),
    ])
}

fn format_anonymous_function_receiver<'source>(
    expression: &AnonymousFunctionExpression<'source>,
) -> Doc<'source> {
    let Some(receiver) = expression.receiver_type() else {
        return jolt_fmt_ir::nil();
    };
    let Some(dot) = expression.dot_token() else {
        return format_type_reference(&receiver);
    };

    concat([
        space(),
        format_type_reference(&receiver),
        format_token(
            &dot,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
    ])
}

fn format_anonymous_function_tail<'source>(
    expression: &AnonymousFunctionExpression<'source>,
) -> Doc<'source> {
    if let Some(block) = expression.block() {
        return concat([space(), format_block(&block)]);
    }

    let Some(assign) = expression.assign_token() else {
        return jolt_fmt_ir::nil();
    };
    let Some(body) = expression.expression() else {
        return concat([
            space(),
            format_token(
                &assign,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
        ]);
    };

    concat([
        space(),
        format_token(
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        space(),
        format_expression(&body),
    ])
}
