use jolt_fmt_ir::{Doc, DocBuilder};
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
    doc: &mut DocBuilder<'source>,
    expression: &AnonymousFunctionExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(fun_token) = expression.fun_token() else {
        return format_token_sequence(doc, expression.token_iter(), leading);
    };

    let fun_token = format_token(
        doc,
        &fun_token,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let receiver = format_anonymous_function_receiver(doc, expression);
    let parameters = if let Some(parameters) = expression.value_parameter_list() {
        format_value_parameter_list(doc, &parameters)
    } else {
        doc.nil()
    };
    let return_type = format_type_annotation(doc, expression.colon(), expression.return_type());
    let tail = format_anonymous_function_tail(doc, expression);
    doc.concat([fun_token, receiver, parameters, return_type, tail])
}

fn format_anonymous_function_receiver<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AnonymousFunctionExpression<'source>,
) -> Doc<'source> {
    let Some(receiver) = expression.receiver_type() else {
        return doc.nil();
    };
    let Some(dot) = expression.dot_token() else {
        return format_type_reference(doc, &receiver);
    };

    let space = doc.space();
    let receiver = format_type_reference(doc, &receiver);
    let dot = format_token(
        doc,
        &dot,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    doc.concat([space, receiver, dot])
}

fn format_anonymous_function_tail<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AnonymousFunctionExpression<'source>,
) -> Doc<'source> {
    if let Some(block) = expression.block() {
        let space = doc.space();
        let block = format_block(doc, &block);
        return doc.concat([space, block]);
    }

    let Some(assign) = expression.assign_token() else {
        return doc.nil();
    };
    let Some(body) = expression.expression() else {
        let space = doc.space();
        let assign = format_token(
            doc,
            &assign,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        return doc.concat([space, assign]);
    };

    let before = doc.space();
    let assign = format_token(
        doc,
        &assign,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let after = doc.space();
    let body = format_expression(doc, &body);
    doc.concat([before, assign, after, body])
}
