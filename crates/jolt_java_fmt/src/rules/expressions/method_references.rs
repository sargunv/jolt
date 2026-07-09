use super::{
    Doc, LeadingTrivia, MethodReferenceExpression, TrailingTrivia, format_array_dimensions,
    format_expression, format_token, format_token_with_comments, format_type,
    format_type_argument_list, trailing_comments_force_line,
};
use jolt_fmt_ir::DocBuilder;

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
                expression
                    .type_arguments()
                    .map_or_else(Doc::nil, |arguments| format_type_argument_list(
                        &arguments, doc
                    ),),
                if expression.is_constructor_reference() {
                    expression
                        .new_token()
                        .map_or_else(Doc::nil, |token| format_token_with_comments(doc, &token))
                } else {
                    expression
                        .target_name()
                        .map_or_else(Doc::nil, |target| format_token_with_comments(doc, &target))
                },
            ]
        )
    )
}

fn format_method_reference_separator<'source>(
    expression: &MethodReferenceExpression<'source>,
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
) -> Doc<'source> {
    expression
        .double_colon()
        .map_or_else(Doc::nil, |separator| {
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
    if let Some(receiver) = expression.receiver_expression() {
        return doc_concat!(
            doc,
            [
                format_expression(&receiver, doc),
                expression
                    .receiver_dimensions()
                    .map_or_else(Doc::nil, |dimensions| format_array_dimensions(
                        &dimensions,
                        doc
                    ),),
            ]
        );
    }

    expression
        .receiver_type()
        .map_or_else(Doc::nil, |ty| format_type(&ty, doc))
}
