use super::{
    CastExpression, Doc, InlineLeadingTrivia, InstanceofExpression, JavaFormatter, JavaSyntaxToken,
    TrailingTrivia, concat, format_expression, format_leading_comments, format_pattern,
    format_token_with_inline_leading_comments, format_trailing_comments,
    format_trailing_comments_before_line_break, format_type, group, hard_line, line, text,
    trailing_comments_force_line,
};

pub(super) fn format_cast_expression<'source>(
    expression: &CastExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open_paren = expression.open_paren();
    let close_paren = expression.close_paren();

    group(concat([
        format_cast_open_paren(open_paren.as_ref()),
        expression
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
        format_cast_close_paren(close_paren.as_ref()),
        if close_paren
            .as_ref()
            .is_some_and(trailing_comments_force_line)
        {
            jolt_fmt_ir::nil()
        } else {
            text(" ")
        },
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
    ]))
}

fn format_cast_open_paren<'source>(open: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    open.map_or_else(
        || text("("),
        |open| {
            format_token_with_inline_leading_comments(
                open,
                InlineLeadingTrivia::BeforeToken,
                TrailingTrivia::BeforeSpaceIfComments,
            )
        },
    )
}

fn format_cast_close_paren<'source>(close: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            jolt_fmt_ir::nil()
        },
        close.map_or_else(
            || text(")"),
            |close| {
                concat([
                    if close_has_leading_comments {
                        format_leading_comments(close)
                    } else {
                        jolt_fmt_ir::nil()
                    },
                    text(")"),
                    format_trailing_comments(close),
                ])
            },
        ),
    ])
}

pub(super) fn format_instanceof_expression<'source>(
    expression: &InstanceofExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        text(" "),
        expression.instanceof_token().map_or_else(
            || text("instanceof "),
            |token| format_instanceof_operator(&token),
        ),
        expression.ty().map_or_else(
            || {
                expression
                    .pattern()
                    .map_or_else(jolt_fmt_ir::nil, |pattern| {
                        format_pattern(&pattern, formatter)
                    })
            },
            |ty| format_type(&ty, formatter),
        ),
    ])
}

fn format_instanceof_operator<'source>(operator: &JavaSyntaxToken<'source>) -> Doc<'source> {
    concat([
        format_leading_comments(operator),
        text("instanceof"),
        format_trailing_comments_before_line_break(operator),
        if trailing_comments_force_line(operator) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}
