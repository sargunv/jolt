use super::{
    Doc, JavaFormatter, MethodReferenceExpression, concat, format_array_dimensions,
    format_expression, format_leading_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, format_type, format_type_argument_list, group,
    hard_line, text, trailing_comments_force_line,
};

pub(super) fn format_method_reference_expression(
    expression: &MethodReferenceExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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
                .map_or_else(|| text("new"), |token| format_token_with_comments(&token))
        } else {
            expression
                .target_name()
                .map_or_else(jolt_fmt_ir::nil, |target| {
                    format_token_with_comments(&target)
                })
        },
    ]))
}

fn format_method_reference_separator(expression: &MethodReferenceExpression) -> Doc {
    expression.double_colon().map_or_else(
        || text("::"),
        |separator| {
            let has_trailing_comments = !separator.trailing_comments().is_empty();
            concat([
                format_leading_comments(&separator),
                text("::"),
                format_trailing_comments_before_line_break(&separator),
                if trailing_comments_force_line(&separator) {
                    hard_line()
                } else if has_trailing_comments {
                    text(" ")
                } else {
                    jolt_fmt_ir::nil()
                },
            ])
        },
    )
}

fn format_method_reference_receiver(
    expression: &MethodReferenceExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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
