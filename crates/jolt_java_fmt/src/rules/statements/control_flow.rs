use super::simple::format_statement_keyword;
use super::{
    BasicForStatement, DoStatement, Doc, EnhancedForStatement, ForInitializer, ForStatement,
    ForUpdate, IfStatement, JavaFormatter, JavaSyntaxToken, Statement, StatementBody,
    StatementExpressionEntry, StatementExpressionList, SynchronizedStatement, WhileStatement,
    comment_forces_line, concat, empty_block, format_block, format_comment, format_expression,
    format_leading_comments, format_local_variable_declaration, format_separator_with_comments,
    format_statement_semicolon, format_token_with_comments,
    format_trailing_comments_before_line_break, group, hard_line, indent, line, semicolon_list,
    soft_line, statement_body_as_block, statement_body_trailing_comments_force_line, text,
    trailing_comments_force_line,
};

pub(super) fn format_if_statement(statement: &IfStatement, formatter: &JavaFormatter<'_>) -> Doc {
    let else_body = statement.else_body();
    let then_body = statement.then_body();
    let then_body_trailing_comments_force_line =
        else_body.is_some() && statement_body_trailing_comments_force_line(then_body.as_ref());
    let open = statement.open_paren();
    let close = statement.close_paren();

    concat([
        format_statement_keyword(statement.keyword(), "if"),
        text(" "),
        format_parenthesized_statement_expression(
            open.as_ref(),
            statement
                .condition()
                .map_or_else(jolt_fmt_ir::nil, |condition| {
                    format_expression(&condition, formatter)
                }),
            close.as_ref(),
        ),
        format_statement_header_body_separator(close.as_ref()),
        statement_body_as_block(then_body.as_ref(), formatter),
        else_body.map_or_else(jolt_fmt_ir::nil, |else_body| {
            concat([
                if then_body_trailing_comments_force_line {
                    jolt_fmt_ir::nil()
                } else {
                    text(" ")
                },
                format_statement_keyword(statement.else_keyword(), "else"),
                text(" "),
                match else_body {
                    StatementBody::Unbraced(Statement::IfStatement(else_if)) => {
                        format_if_statement(&else_if, formatter)
                    }
                    body => statement_body_as_block(Some(&body), formatter),
                },
            ])
        }),
    ])
}

pub(super) fn format_parenthesized_statement_expression(
    open: Option<&JavaSyntaxToken>,
    expression: Doc,
    close: Option<&JavaSyntaxToken>,
) -> Doc {
    group(concat([
        format_condition_open_paren(open),
        indent(concat([format_condition_open_spacing(open), expression])),
        format_condition_close_paren(close),
    ]))
}

pub(super) fn format_condition_open_paren(open: Option<&JavaSyntaxToken>) -> Doc {
    open.map_or_else(
        || text("("),
        |open| concat([format_leading_comments(open), text("(")]),
    )
}

fn format_condition_open_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
    let Some(open) = open else {
        return jolt_fmt_ir::nil();
    };

    if open.trailing_comments().is_empty() {
        return soft_line();
    }

    concat([
        format_trailing_comments_before_line_break(open),
        if trailing_comments_force_line(open) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_condition_close_paren(close: Option<&JavaSyntaxToken>) -> Doc {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
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
                    format_trailing_comments_before_line_break(close),
                    if trailing_comments_force_line(close) {
                        hard_line()
                    } else {
                        jolt_fmt_ir::nil()
                    },
                ])
            },
        ),
    ])
}

pub(super) fn format_statement_header_body_separator(close: Option<&JavaSyntaxToken>) -> Doc {
    if close.is_some_and(trailing_comments_force_line) {
        jolt_fmt_ir::nil()
    } else {
        text(" ")
    }
}

pub(super) fn format_while_statement(
    statement: &WhileStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "while"),
        text(" "),
        format_parenthesized_statement_expression(
            open.as_ref(),
            statement
                .condition()
                .map_or_else(jolt_fmt_ir::nil, |condition| {
                    format_expression(&condition, formatter)
                }),
            close.as_ref(),
        ),
        format_statement_header_body_separator(close.as_ref()),
        statement_body_as_block(statement.statement_body().as_ref(), formatter),
    ])
}

pub(super) fn format_do_statement(statement: &DoStatement, formatter: &JavaFormatter<'_>) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "do"),
        text(" "),
        statement_body_as_block(statement.statement_body().as_ref(), formatter),
        text(" "),
        format_statement_keyword(statement.while_keyword(), "while"),
        text(" "),
        format_parenthesized_statement_expression(
            open.as_ref(),
            statement
                .condition()
                .map_or_else(jolt_fmt_ir::nil, |condition| {
                    format_expression(&condition, formatter)
                }),
            close.as_ref(),
        ),
        format_statement_semicolon(statement.semicolon()),
    ])
}

pub(super) fn format_for_statement(statement: &ForStatement, formatter: &JavaFormatter<'_>) -> Doc {
    if let Some(basic) = statement.basic() {
        return format_basic_for_statement(&basic, formatter);
    }
    if let Some(enhanced) = statement.enhanced() {
        return format_enhanced_for_statement(&enhanced, formatter);
    }

    jolt_fmt_ir::nil()
}

fn format_basic_for_statement(statement: &BasicForStatement, formatter: &JavaFormatter<'_>) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    let initializer = statement
        .initializer()
        .map(|initializer| format_for_initializer(&initializer, formatter));
    let condition = statement
        .condition()
        .map(|condition| format_expression(&condition, formatter));
    let update = statement
        .update()
        .map(|update| format_for_update(&update, formatter));
    let is_empty_header = initializer.is_none() && condition.is_none() && update.is_none();
    let header = if is_empty_header {
        concat([
            format_statement_keyword(statement.keyword(), "for"),
            text(" "),
            format_condition_open_paren(open.as_ref()),
            format_inline_open_paren_spacing(open.as_ref()),
            format_for_header_semicolon(statement.first_semicolon()),
            format_for_header_semicolon(statement.second_semicolon()),
            format_inline_close_paren(close.as_ref()),
        ])
    } else {
        group(concat([
            format_statement_keyword(statement.keyword(), "for"),
            text(" "),
            format_condition_open_paren(open.as_ref()),
            indent(concat([
                format_for_header_open_spacing(open.as_ref()),
                semicolon_list(vec![
                    initializer.unwrap_or_else(jolt_fmt_ir::nil),
                    condition.unwrap_or_else(jolt_fmt_ir::nil),
                    update.unwrap_or_else(jolt_fmt_ir::nil),
                ]),
            ])),
            format_for_header_close_paren(close.as_ref()),
        ]))
    };

    concat([
        header,
        format_statement_header_body_separator(close.as_ref()),
        statement_body_as_block(statement.statement_body().as_ref(), formatter),
    ])
}

fn format_enhanced_for_statement(
    statement: &EnhancedForStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "for"),
        text(" "),
        group(concat([
            format_condition_open_paren(open.as_ref()),
            indent(concat([
                format_for_header_open_spacing(open.as_ref()),
                statement
                    .variable()
                    .map_or_else(jolt_fmt_ir::nil, |variable| {
                        format_local_variable_declaration(&variable, formatter)
                    }),
                text(" "),
                statement
                    .colon()
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                text(" "),
                statement
                    .iterable()
                    .map_or_else(jolt_fmt_ir::nil, |iterable| {
                        format_expression(&iterable, formatter)
                    }),
            ])),
            format_for_header_close_paren(close.as_ref()),
        ])),
        format_statement_header_body_separator(close.as_ref()),
        statement_body_as_block(statement.statement_body().as_ref(), formatter),
    ])
}

fn format_for_header_open_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
    if open.is_some_and(|open| !open.trailing_comments().is_empty()) {
        format_condition_open_spacing(open)
    } else {
        soft_line()
    }
}

fn format_inline_close_paren(close: Option<&JavaSyntaxToken>) -> Doc {
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
                    format_trailing_comments_before_line_break(close),
                    if trailing_comments_force_line(close) {
                        hard_line()
                    } else {
                        jolt_fmt_ir::nil()
                    },
                ])
            },
        ),
    ])
}

fn format_inline_open_paren_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
    let Some(open) = open else {
        return jolt_fmt_ir::nil();
    };

    if open.trailing_comments().is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        format_trailing_comments_before_line_break(open),
        if trailing_comments_force_line(open) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_for_header_semicolon(semicolon: Option<JavaSyntaxToken>) -> Doc {
    let Some(semicolon) = semicolon else {
        return text(";");
    };

    concat([
        format_for_header_semicolon_leading_comments(&semicolon),
        text(";"),
        format_trailing_comments_before_line_break(&semicolon),
        if trailing_comments_force_line(&semicolon) {
            hard_line()
        } else {
            jolt_fmt_ir::nil()
        },
    ])
}

fn format_for_header_semicolon_leading_comments(semicolon: &JavaSyntaxToken) -> Doc {
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

fn format_for_header_close_paren(close: Option<&JavaSyntaxToken>) -> Doc {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
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
                    format_trailing_comments_before_line_break(close),
                    if trailing_comments_force_line(close) {
                        hard_line()
                    } else {
                        jolt_fmt_ir::nil()
                    },
                ])
            },
        ),
    ])
}

fn format_for_initializer(initializer: &ForInitializer, formatter: &JavaFormatter<'_>) -> Doc {
    if let Some(declaration) = initializer.local_variable_declaration() {
        return format_local_variable_declaration(&declaration, formatter);
    }
    initializer
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions, formatter)
        })
}

fn format_for_update(update: &ForUpdate, formatter: &JavaFormatter<'_>) -> Doc {
    update
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions, formatter)
        })
}

fn format_statement_expression_list(
    expressions: &StatementExpressionList,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_statement_expression_entries(expressions.entries().collect(), formatter)
}

fn format_statement_expression_entries(
    entries: Vec<StatementExpressionEntry>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_expression(&entry.expression, formatter));
        if let Some(comma) = entry.comma {
            docs.push(format_separator_with_comments(&comma, text(" ")));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

pub(super) fn format_synchronized_statement(
    statement: &SynchronizedStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "synchronized"),
        text(" "),
        format_parenthesized_statement_expression(
            open.as_ref(),
            statement
                .expression()
                .map_or_else(jolt_fmt_ir::nil, |expression| {
                    format_expression(&expression, formatter)
                }),
            close.as_ref(),
        ),
        format_statement_header_body_separator(close.as_ref()),
        statement
            .body()
            .map_or_else(empty_block, |body| format_block(&body, formatter)),
    ])
}
