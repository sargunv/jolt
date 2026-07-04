use super::{
    Doc, JavaFormatter, LeadingTrivia, ParenthesizedExpression, TrailingTrivia,
    comment_forces_line, concat, format_expression, format_token, format_token_with_comments,
    format_trailing_comments_before_line_break, group, hard_line, indent, line, soft_line, text,
};

pub(super) fn format_parenthesized_expression<'source>(
    expression: &ParenthesizedExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    group(concat([
        format_parenthesized_expression_open(expression),
        indent(concat([
            format_open_parenthesized_expression_spacing(expression),
            expression
                .expression()
                .map_or_else(jolt_fmt_ir::nil, |expression| {
                    format_expression(&expression, formatter)
                }),
        ])),
        format_parenthesized_expression_close_with_spacing(expression),
    ]))
}

fn format_parenthesized_expression_open<'source>(
    expression: &ParenthesizedExpression<'source>,
) -> Doc<'source> {
    expression
        .open_paren()
        .map_or_else(jolt_fmt_ir::nil, |open| {
            format_token(
                &open,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        })
}

fn format_open_parenthesized_expression_spacing<'source>(
    expression: &ParenthesizedExpression<'source>,
) -> Doc<'source> {
    let Some(open) = expression.open_paren() else {
        return soft_line();
    };

    if open.trailing_comments().is_empty() {
        return soft_line();
    }

    concat([
        format_trailing_comments_before_line_break(&open),
        if open
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
        {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_parenthesized_expression_close_with_spacing<'source>(
    expression: &ParenthesizedExpression<'source>,
) -> Doc<'source> {
    let close_has_leading_comments = expression
        .close_paren()
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        expression
            .close_paren()
            .map_or_else(jolt_fmt_ir::nil, |close| format_token_with_comments(&close)),
    ])
}
