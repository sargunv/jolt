use super::simple::format_statement_keyword;
use super::{
    BasicForStatement, DoStatement, Doc, EnhancedForStatement, ForInitializer, ForStatement,
    ForUpdate, IfStatement, JavaFormatter, JavaSyntaxToken, LeadingTrivia, Statement,
    StatementBody, StatementExpressionList, SynchronizedStatement, TrailingTrivia, WhileStatement,
    concat, empty_block, format_block, format_expression, format_local_variable_declaration,
    format_separator_with_comments, format_statement_semicolon, format_token,
    format_token_sequence, format_token_with_comments, format_trailing_comments_before_line_break,
    group, hard_line, indent, line, soft_line, statement_body_as_block,
    statement_body_trailing_comments_force_line, trailing_comments_force_line,
};
use jolt_fmt_ir::space;

pub(super) fn format_if_statement<'source>(
    statement: &IfStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let else_body = statement.else_body();
    let then_body = statement.then_body();
    let then_body_trailing_comments_force_line =
        else_body.is_some() && statement_body_trailing_comments_force_line(then_body.as_ref());
    let open = statement.open_paren();
    let close = statement.close_paren();

    concat([
        format_statement_keyword(statement.keyword(), "if"),
        space(),
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
                    space()
                },
                format_statement_keyword(statement.else_keyword(), "else"),
                space(),
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

pub(super) fn format_parenthesized_statement_expression<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    expression: Doc<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    group(concat([
        format_condition_open_paren(open),
        indent(concat([format_condition_open_spacing(open), expression])),
        format_condition_close_paren(close),
    ]))
}

pub(super) fn format_condition_open_paren<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    open.map_or_else(jolt_fmt_ir::nil, |open| {
        format_token(
            open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    })
}

fn format_condition_open_spacing<'source>(open: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
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
            space()
        },
    ])
}

fn format_condition_close_paren<'source>(close: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        close.map_or_else(jolt_fmt_ir::nil, |close| {
            concat([
                format_token(
                    close,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak,
                ),
                if trailing_comments_force_line(close) {
                    hard_line()
                } else {
                    jolt_fmt_ir::nil()
                },
            ])
        }),
    ])
}

pub(super) fn format_statement_header_body_separator<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    if close.is_some_and(trailing_comments_force_line) {
        jolt_fmt_ir::nil()
    } else {
        space()
    }
}

pub(super) fn format_while_statement<'source>(
    statement: &WhileStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "while"),
        space(),
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

pub(super) fn format_do_statement<'source>(
    statement: &DoStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "do"),
        space(),
        statement_body_as_block(statement.statement_body().as_ref(), formatter),
        space(),
        format_statement_keyword(statement.while_keyword(), "while"),
        space(),
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

pub(super) fn format_for_statement<'source>(
    statement: &ForStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if let Some(basic) = statement.basic() {
        return format_basic_for_statement(&basic, formatter);
    }
    if let Some(enhanced) = statement.enhanced() {
        return format_enhanced_for_statement(&enhanced, formatter);
    }

    jolt_fmt_ir::nil()
}

fn format_basic_for_statement<'source>(
    statement: &BasicForStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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
            space(),
            format_condition_open_paren(open.as_ref()),
            format_inline_open_paren_spacing(open.as_ref()),
            format_for_header_semicolon(statement.first_semicolon()),
            format_for_header_semicolon(statement.second_semicolon()),
            format_inline_close_paren(close.as_ref()),
        ])
    } else {
        group(concat([
            format_statement_keyword(statement.keyword(), "for"),
            space(),
            format_condition_open_paren(open.as_ref()),
            indent(concat([
                format_for_header_open_spacing(open.as_ref()),
                format_basic_for_header_clauses(
                    initializer,
                    statement.first_semicolon(),
                    condition,
                    statement.second_semicolon(),
                    update,
                ),
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

fn format_basic_for_header_clauses<'source>(
    initializer: Option<Doc<'source>>,
    first_semicolon: Option<JavaSyntaxToken<'source>>,
    condition: Option<Doc<'source>>,
    second_semicolon: Option<JavaSyntaxToken<'source>>,
    update: Option<Doc<'source>>,
) -> Doc<'source> {
    concat([
        initializer.unwrap_or_else(jolt_fmt_ir::nil),
        first_semicolon.map_or_else(jolt_fmt_ir::nil, |semicolon| {
            format_separator_with_comments(&semicolon, line())
        }),
        condition.unwrap_or_else(jolt_fmt_ir::nil),
        second_semicolon.map_or_else(jolt_fmt_ir::nil, |semicolon| {
            format_separator_with_comments(&semicolon, line())
        }),
        update.unwrap_or_else(jolt_fmt_ir::nil),
    ])
}

fn format_enhanced_for_statement<'source>(
    statement: &EnhancedForStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "for"),
        space(),
        group(concat([
            format_condition_open_paren(open.as_ref()),
            indent(concat([
                format_for_header_open_spacing(open.as_ref()),
                statement
                    .variable()
                    .map_or_else(jolt_fmt_ir::nil, |variable| {
                        format_local_variable_declaration(&variable, formatter)
                    }),
                space(),
                statement
                    .colon()
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                space(),
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

fn format_for_header_open_spacing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    if open.is_some_and(|open| !open.trailing_comments().is_empty()) {
        format_condition_open_spacing(open)
    } else {
        soft_line()
    }
}

fn format_inline_close_paren<'source>(close: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            jolt_fmt_ir::nil()
        },
        close.map_or_else(jolt_fmt_ir::nil, |close| {
            concat([
                format_token(
                    close,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak,
                ),
                if trailing_comments_force_line(close) {
                    hard_line()
                } else {
                    jolt_fmt_ir::nil()
                },
            ])
        }),
    ])
}

fn format_inline_open_paren_spacing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
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
            space()
        },
    ])
}

fn format_for_header_semicolon(semicolon: Option<JavaSyntaxToken<'_>>) -> Doc<'_> {
    let Some(semicolon) = semicolon else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_token(
            &semicolon,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
        if trailing_comments_force_line(&semicolon) {
            hard_line()
        } else {
            jolt_fmt_ir::nil()
        },
    ])
}

fn format_for_header_close_paren<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        close.map_or_else(jolt_fmt_ir::nil, |close| {
            concat([
                format_token(
                    close,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak,
                ),
                if trailing_comments_force_line(close) {
                    hard_line()
                } else {
                    jolt_fmt_ir::nil()
                },
            ])
        }),
    ])
}

fn format_for_initializer<'source>(
    initializer: &ForInitializer<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if let Some(declaration) = initializer.local_variable_declaration() {
        return format_local_variable_declaration(&declaration, formatter);
    }
    initializer
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions, formatter)
        })
}

fn format_for_update<'source>(
    update: &ForUpdate<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    update
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions, formatter)
        })
}

fn format_statement_expression_list<'source>(
    expressions: &StatementExpressionList<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_statement_expression_entries(statement_expression_parts(expressions, formatter))
}

enum StatementExpressionPart<'source> {
    Expression {
        expression: Doc<'source>,
        comma: Option<JavaSyntaxToken<'source>>,
    },
    Recovered(Doc<'source>),
}

fn statement_expression_parts<'source, 'fmt>(
    expressions: &'fmt StatementExpressionList<'source>,
    formatter: &'fmt JavaFormatter<'_>,
) -> impl Iterator<Item = StatementExpressionPart<'source>> + use<'source, 'fmt> {
    expressions
        .entries_with_recovered()
        .map(move |entry| match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => {
                StatementExpressionPart::Expression {
                    expression: format_expression(&entry.expression, formatter),
                    comma: entry.comma,
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                StatementExpressionPart::Recovered(format_token(
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ))
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                StatementExpressionPart::Recovered(format_token_sequence(
                    error.token_iter(),
                    LeadingTrivia::Preserve,
                ))
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                StatementExpressionPart::Recovered(format_token_sequence(
                    node.token_iter(),
                    LeadingTrivia::Preserve,
                ))
            }
        })
}

fn format_statement_expression_entries<'source>(
    entries: impl IntoIterator<Item = StatementExpressionPart<'source>>,
) -> Doc<'source> {
    let mut entries = entries.into_iter().peekable();
    let (lower, _) = entries.size_hint();
    let mut docs = Vec::with_capacity(lower.saturating_mul(2));

    while let Some(entry) = entries.next() {
        let has_next = entries.peek().is_some();
        match entry {
            StatementExpressionPart::Expression { expression, comma } => {
                docs.push(expression);
                if let Some(comma) = comma {
                    docs.push(format_separator_with_comments(&comma, space()));
                } else if has_next {
                    docs.push(line());
                }
            }
            StatementExpressionPart::Recovered(doc) => {
                docs.push(doc);
                if has_next {
                    docs.push(line());
                }
            }
        }
    }

    concat(docs)
}

pub(super) fn format_synchronized_statement<'source>(
    statement: &SynchronizedStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "synchronized"),
        space(),
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
