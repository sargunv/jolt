use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{AnonymousFunctionExpression, Expression, KotlinRoleElement};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::recovery::{
    KotlinFormatField, format_optional_field, format_or_verbatim, format_required_field,
    resolve_optional_field,
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
    format_or_verbatim(expression, doc, |doc| {
        let fun_token = format_required_field(expression.fun_token(), doc, |token, doc| {
            format_token(
                doc,
                &token,
                leading,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        });
        let receiver = format_anonymous_function_receiver(doc, expression);
        let parameters = format_required_field(expression.parameters(), doc, |parameters, doc| {
            format_value_parameter_list(doc, &parameters)
        });
        let return_type =
            format_type_annotation(doc, expression.return_colon(), expression.return_type());
        let tail = format_anonymous_function_tail(doc, expression);
        doc.concat([fun_token, receiver, parameters, return_type, tail])
    })
}

fn format_anonymous_function_receiver<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AnonymousFunctionExpression<'source>,
) -> Doc<'source> {
    let receiver = match resolve_optional_field(expression.receiver(), doc) {
        KotlinFormatField::Present(Some(receiver)) => receiver,
        KotlinFormatField::Present(None) => return Doc::nil(),
        KotlinFormatField::Malformed(recovery) => return recovery,
    };
    let receiver = format_type_reference(doc, &receiver);
    let dot = format_optional_field(expression.dot(), doc, |dot, doc| {
        format_token(
            doc,
            &dot,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let space = doc.space();
    doc.concat([space, receiver, dot])
}

fn format_anonymous_function_tail<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AnonymousFunctionExpression<'source>,
) -> Doc<'source> {
    let (assign, has_assign) = match resolve_optional_field(expression.assign(), doc) {
        KotlinFormatField::Present(Some(assign)) => (
            format_token(
                doc,
                &assign,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            true,
        ),
        KotlinFormatField::Present(None) => (Doc::nil(), false),
        KotlinFormatField::Malformed(recovery) => (recovery, true),
    };
    match resolve_optional_field(expression.body(), doc) {
        KotlinFormatField::Present(Some(body)) => {
            format_anonymous_body(doc, body, assign, has_assign)
        }
        KotlinFormatField::Present(None) => {
            if has_assign {
                let space = doc.space();
                doc.concat([space, assign])
            } else {
                Doc::nil()
            }
        }
        KotlinFormatField::Malformed(recovery) => doc.concat([assign, recovery]),
    }
}

fn format_anonymous_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: KotlinRoleElement<'source>,
    assign: Doc<'source>,
    has_assign: bool,
) -> Doc<'source> {
    if let Some(block) = body.cast_node::<jolt_kotlin_syntax::Block<'source>>() {
        let block = format_block(doc, &block);
        let space = doc.space();
        return if has_assign {
            doc.concat([space, assign, space, block])
        } else {
            doc.concat([space, block])
        };
    }
    if let Some(expression) = body.cast_family::<Expression<'source>>() {
        let before = doc.space();
        let body = format_expression(doc, &expression);
        return if has_assign {
            let after = doc.space();
            doc.concat([before, assign, after, body])
        } else {
            doc.concat([before, body])
        };
    }
    doc.block_on_invariant("Kotlin anonymous function body contained an unsupported element");
    Doc::nil()
}
