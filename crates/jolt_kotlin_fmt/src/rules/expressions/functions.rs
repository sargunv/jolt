use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::AnonymousFunctionExpression;

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::recovery::{
    KotlinFormatField, format_optional_field, format_required_field, resolve_optional_field,
};
use crate::rules::declarations::{format_declaration_body, format_type_annotation};
use crate::rules::types::format_type_reference;
use crate::rules::variables::format_value_parameter_list;

pub(super) fn format_anonymous_function_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AnonymousFunctionExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
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
    let tail = format_required_field(expression.body(), doc, |body, doc| {
        format_declaration_body(doc, &body)
    });
    doc.concat([fun_token, receiver, parameters, return_type, tail])
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
