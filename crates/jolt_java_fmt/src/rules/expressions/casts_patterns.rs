use super::{
    CastExpression, Doc, InstanceofExpression, JavaFormatter, JavaSyntaxToken, comment_forces_line,
    concat, format_expression, format_leading_comments, format_pattern, format_trailing_comments,
    format_trailing_comments_before_line_break, format_type, hard_line, line, text,
    trailing_comments_force_line,
};

pub(super) fn format_cast_expression(
    expression: &CastExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open_paren = expression.open_paren();
    let close_paren = expression.close_paren();

    concat([
        format_cast_open_paren(open_paren.as_ref()),
        format_cast_open_paren_spacing(open_paren.as_ref()),
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
    ])
}

fn format_cast_open_paren(open: Option<&JavaSyntaxToken>) -> Doc {
    open.map_or_else(
        || text("("),
        |open| concat([format_leading_comments(open), text("(")]),
    )
}

fn format_cast_open_paren_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
    let Some(open) = open else {
        return jolt_fmt_ir::nil();
    };

    if open.trailing_comments().is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        format_trailing_comments_before_line_break(open),
        if open.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_cast_close_paren(close: Option<&JavaSyntaxToken>) -> Doc {
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

pub(super) fn format_instanceof_expression(
    expression: &InstanceofExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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

fn format_instanceof_operator(operator: &JavaSyntaxToken) -> Doc {
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
