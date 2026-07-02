use super::{
    AssertStatement, Doc, Expression, ExpressionStatement, JavaComment, JavaFormatter,
    JavaSyntaxToken, LabeledStatement, ReturnStatement, ThrowStatement, YieldStatement,
    comment_forces_line, comment_is_star_block, concat, format_comment, format_expression,
    format_leading_comments, format_statement, format_token_text, format_token_with_comments,
    format_trailing_comments_before_line_break, hard_line, indent, text,
    trailing_comments_force_line,
};

pub(super) fn format_labeled_statement(
    statement: &LabeledStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let label = statement
        .label()
        .map_or_else(jolt_fmt_ir::nil, |label| format_token_with_comments(&label));

    concat([
        label,
        text(":"),
        hard_line(),
        statement
            .body()
            .map_or_else(jolt_fmt_ir::nil, |body| format_statement(&body, formatter)),
    ])
}

pub(super) fn format_expression_statement(
    statement: &ExpressionStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        statement
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        format_statement_semicolon(statement.semicolon()),
    ])
}
pub(super) fn format_assert_statement(
    statement: &AssertStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        format_statement_keyword(statement.keyword(), "assert"),
        text(" "),
        statement
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| {
                format_expression(&condition, formatter)
            }),
        statement.detail().map_or_else(jolt_fmt_ir::nil, |detail| {
            concat([text(" : "), format_expression(&detail, formatter)])
        }),
        format_statement_semicolon(statement.semicolon()),
    ])
}

pub(super) fn format_return_statement(
    statement: &ReturnStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "return",
        statement.expression(),
        statement.semicolon(),
        formatter,
    )
}

pub(super) fn format_throw_statement(
    statement: &ThrowStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "throw",
        statement.expression(),
        statement.semicolon(),
        formatter,
    )
}

pub(super) fn format_yield_statement(
    statement: &YieldStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "yield",
        statement.expression(),
        statement.semicolon(),
        formatter,
    )
}

fn format_keyword_expression_statement(
    keyword: Option<&JavaSyntaxToken>,
    fallback: &'static str,
    expression: Option<Expression>,
    semicolon: Option<JavaSyntaxToken>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        format_statement_keyword_head(keyword, fallback),
        expression.map_or_else(jolt_fmt_ir::nil, |expression| {
            let expression_doc = concat([
                format_keyword_expression_separator(keyword),
                format_expression(&expression, formatter),
            ]);
            if matches!(expression, Expression::SwitchExpression(_)) {
                expression_doc
            } else {
                indent(expression_doc)
            }
        }),
        format_statement_semicolon(semicolon),
    ])
}

fn format_keyword_expression_separator(keyword: Option<&JavaSyntaxToken>) -> Doc {
    let Some(keyword) = keyword else {
        return text(" ");
    };

    if keyword.trailing_comments().is_empty() {
        return text(" ");
    }

    concat([
        format_trailing_comments_before_line_break(keyword),
        if trailing_comments_force_line(keyword) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

pub(super) fn format_jump_statement(
    keyword: Option<JavaSyntaxToken>,
    fallback: &'static str,
    label: Option<JavaSyntaxToken>,
    semicolon: Option<JavaSyntaxToken>,
) -> Doc {
    concat([
        format_statement_keyword(keyword, fallback),
        label.map_or_else(jolt_fmt_ir::nil, |label| {
            concat([text(" "), format_token_with_comments(&label)])
        }),
        format_statement_semicolon(semicolon),
    ])
}

pub(crate) fn format_statement_semicolon(semicolon: Option<JavaSyntaxToken>) -> Doc {
    let Some(semicolon) = semicolon else {
        return text(";");
    };

    concat([
        format_semicolon_leading_comments(&semicolon),
        text(";"),
        format_terminator_trailing_comments(&semicolon),
    ])
}

fn format_semicolon_leading_comments(semicolon: &JavaSyntaxToken) -> Doc {
    let mut docs = Vec::new();
    for comment in semicolon.leading_comments() {
        docs.push(text(" "));
        docs.push(format_comment(&comment));
        if comment_forces_line(&comment) {
            docs.push(hard_line());
        }
    }
    concat(docs)
}

fn format_terminator_trailing_comments(token: &JavaSyntaxToken) -> Doc {
    let mut docs = Vec::new();
    for comment in token.trailing_comments() {
        if terminator_comment_starts_next_line(&comment) {
            docs.push(hard_line());
        } else {
            docs.push(text(" "));
        }
        docs.push(format_comment(&comment));
    }
    concat(docs)
}

fn terminator_comment_starts_next_line(comment: &JavaComment) -> bool {
    comment_is_star_block(comment)
}

pub(super) fn format_statement_keyword(
    keyword: Option<JavaSyntaxToken>,
    fallback: &'static str,
) -> Doc {
    keyword.map_or_else(
        || text(fallback),
        |keyword| format_token_with_comments(&keyword),
    )
}

pub(super) fn format_statement_keyword_head(
    keyword: Option<&JavaSyntaxToken>,
    fallback: &'static str,
) -> Doc {
    keyword.map_or_else(
        || text(fallback),
        |keyword| {
            concat([
                format_leading_comments(keyword),
                format_token_text(keyword.text()),
            ])
        },
    )
}
