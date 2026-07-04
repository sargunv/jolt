use super::{
    Doc, JavaFormatter, LeadingTrivia, MethodReferenceExpression, TrailingTrivia, concat,
    format_array_dimensions, format_expression, format_token, format_token_with_comments,
    format_type, format_type_argument_list, group, hard_line, text, trailing_comments_force_line,
};

pub(super) fn format_method_reference_expression<'source>(
    expression: &MethodReferenceExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    group(concat([
        format_method_reference_receiver(expression, formatter),
        format_method_reference_separator(expression),
        expression
            .type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments, formatter)
            }),
        if expression.is_constructor_reference() {
            expression
                .new_token()
                .map_or_else(jolt_fmt_ir::nil, |token| format_token_with_comments(&token))
        } else {
            expression
                .target_name()
                .map_or_else(jolt_fmt_ir::nil, |target| {
                    format_token_with_comments(&target)
                })
        },
    ]))
}

fn format_method_reference_separator<'source>(
    expression: &MethodReferenceExpression<'source>,
) -> Doc<'source> {
    expression
        .double_colon()
        .map_or_else(jolt_fmt_ir::nil, |separator| {
            let has_trailing_comments = !separator.trailing_comments().is_empty();
            concat([
                format_token(
                    &separator,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak,
                ),
                if trailing_comments_force_line(&separator) {
                    hard_line()
                } else if has_trailing_comments {
                    text(" ")
                } else {
                    jolt_fmt_ir::nil()
                },
            ])
        })
}

fn format_method_reference_receiver<'source>(
    expression: &MethodReferenceExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if let Some(receiver) = expression.receiver_expression() {
        return concat([
            format_expression(&receiver, formatter),
            expression
                .receiver_dimensions()
                .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                    format_array_dimensions(&dimensions, formatter)
                }),
        ]);
    }

    expression
        .receiver_type()
        .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter))
}
