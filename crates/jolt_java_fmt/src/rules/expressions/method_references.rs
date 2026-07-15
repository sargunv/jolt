use super::{
    Doc, Expression, LeadingTrivia, MethodReferenceExpression, TrailingTrivia, format_expression,
    format_token, format_token_with_comments, format_type, format_type_argument_list,
    trailing_comments_force_line,
};
use crate::helpers::recovery::{format_optional_field, format_required_field};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::Type;

pub(super) fn format_method_reference_expression<'source>(
    expression: &MethodReferenceExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_method_reference_receiver(expression, doc),
                format_method_reference_separator(expression, doc),
                format_optional_field(
                    expression.receiver_type_arguments(),
                    doc,
                    |arguments, doc| format_type_argument_list(&arguments, doc),
                ),
                format_optional_field(expression.target_type_arguments(), doc, |arguments, doc| {
                    format_type_argument_list(&arguments, doc)
                }),
                format_required_field(expression.target(), doc, |target, doc| {
                    format_token_with_comments(doc, &target)
                }),
            ]
        )
    )
}

fn format_method_reference_separator<'source>(
    expression: &MethodReferenceExpression<'source>,
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(expression.double_colon(), doc, |separator, doc| {
        let has_trailing_comments = !separator.trailing_comments().is_empty();
        doc_concat!(
            doc,
            [
                format_token(
                    doc,
                    &separator,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak,
                ),
                if trailing_comments_force_line(&separator) {
                    doc.hard_line()
                } else if has_trailing_comments {
                    doc.space()
                } else {
                    Doc::nil()
                },
            ]
        )
    })
}

fn format_method_reference_receiver<'source>(
    expression: &MethodReferenceExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(expression.receiver(), doc, |receiver, doc| {
        if let Some(expression) = receiver.cast_family::<Expression<'source>>() {
            format_expression(&expression, doc)
        } else if let Some(ty) = receiver.cast_family::<Type<'source>>() {
            format_type(&ty, doc)
        } else {
            doc.block_on_invariant("method reference receiver was neither expression nor type");
            Doc::nil()
        }
    })
}
