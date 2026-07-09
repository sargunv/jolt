use super::simple::format_statement_keyword;
use super::{
    BasicForStatement, DoStatement, Doc, EnhancedForStatement, ForInitializer, ForStatement,
    ForUpdate, IfStatement, JavaSyntaxToken, LeadingTrivia, Statement, StatementBody,
    StatementExpressionList, SynchronizedStatement, TrailingTrivia, WhileStatement, empty_block,
    format_block, format_expression, format_local_variable_declaration,
    format_separator_with_comments, format_statement_semicolon, format_token,
    format_token_sequence, format_token_with_comments, format_trailing_comments_before_line_break,
    statement_body_as_block, statement_body_trailing_comments_force_line,
    trailing_comments_force_line,
};
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_if_statement<'source>(
    statement: &IfStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let else_body = statement.else_body();
    let then_body = statement.then_body();
    let then_body_trailing_comments_force_line =
        else_body.is_some() && statement_body_trailing_comments_force_line(then_body.as_ref());
    let open = statement.open_paren();
    let close = statement.close_paren();
    let condition = match statement.condition() {
        Some(condition) => format_expression(&condition, doc),
        None => Doc::nil(),
    };
    let condition =
        format_parenthesized_statement_expression(doc, open.as_ref(), condition, close.as_ref());
    let then_body = statement_body_as_block(then_body.as_ref(), doc);
    let else_body = match else_body {
        Some(else_body) => {
            let separator = if then_body_trailing_comments_force_line {
                Doc::nil()
            } else {
                doc.space()
            };
            let else_keyword = format_statement_keyword(statement.else_keyword(), "else", doc);
            let body = match else_body {
                StatementBody::Unbraced(Statement::IfStatement(else_if)) => {
                    format_if_statement(&else_if, doc)
                }
                body => statement_body_as_block(Some(&body), doc),
            };
            doc_concat!(doc, [separator, else_keyword, doc.space(), body])
        }
        None => Doc::nil(),
    };

    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.keyword(), "if", doc),
            doc.space(),
            condition,
            format_statement_header_body_separator(close.as_ref(), doc),
            then_body,
            else_body,
        ]
    )
}

pub(super) fn format_parenthesized_statement_expression<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    expression: Doc<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let open_spacing = format_condition_open_spacing(open, doc);
    let open = format_condition_open_paren(open, doc);
    let indented = doc_concat!(doc, [open_spacing, expression]);
    let indented = doc_indent!(doc, indented);
    let close = format_condition_close_paren(close, doc);
    doc_group!(doc, doc_concat!(doc, [open, indented, close]))
}

pub(super) fn format_condition_open_paren<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    open.map_or_else(Doc::nil, |open| {
        format_token(
            doc,
            open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    })
}

fn format_condition_open_spacing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(open) = open else {
        return Doc::nil();
    };

    if open.trailing_comments().is_empty() {
        return doc.soft_line();
    }

    doc_concat!(
        doc,
        [
            format_trailing_comments_before_line_break(doc, open),
            if trailing_comments_force_line(open) {
                doc.hard_line()
            } else {
                doc.space()
            },
        ]
    )
}

fn format_condition_close_paren<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    doc_concat!(
        doc,
        [
            if close_has_leading_comments {
                doc.line()
            } else {
                doc.soft_line()
            },
            close.map_or_else(Doc::nil, |close| {
                doc_concat!(
                    doc,
                    [
                        format_token(
                            doc,
                            close,
                            LeadingTrivia::Preserve,
                            TrailingTrivia::BeforeLineBreak,
                        ),
                        if trailing_comments_force_line(close) {
                            doc.hard_line()
                        } else {
                            Doc::nil()
                        },
                    ]
                )
            },),
        ]
    )
}

pub(super) fn format_statement_header_body_separator<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if close.is_some_and(trailing_comments_force_line) {
        Doc::nil()
    } else {
        doc.space()
    }
}

pub(super) fn format_while_statement<'source>(
    statement: &WhileStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    let condition = match statement.condition() {
        Some(condition) => format_expression(&condition, doc),
        None => Doc::nil(),
    };
    let condition =
        format_parenthesized_statement_expression(doc, open.as_ref(), condition, close.as_ref());
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.keyword(), "while", doc),
            doc.space(),
            condition,
            format_statement_header_body_separator(close.as_ref(), doc),
            statement_body_as_block(statement.statement_body().as_ref(), doc),
        ]
    )
}

pub(super) fn format_do_statement<'source>(
    statement: &DoStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    let condition = match statement.condition() {
        Some(condition) => format_expression(&condition, doc),
        None => Doc::nil(),
    };
    let condition =
        format_parenthesized_statement_expression(doc, open.as_ref(), condition, close.as_ref());
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.keyword(), "do", doc),
            doc.space(),
            statement_body_as_block(statement.statement_body().as_ref(), doc),
            doc.space(),
            format_statement_keyword(statement.while_keyword(), "while", doc),
            doc.space(),
            condition,
            format_statement_semicolon(statement.semicolon(), doc),
        ]
    )
}

pub(super) fn format_for_statement<'source>(
    statement: &ForStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(basic) = statement.basic() {
        return format_basic_for_statement(&basic, doc);
    }
    if let Some(enhanced) = statement.enhanced() {
        return format_enhanced_for_statement(&enhanced, doc);
    }

    Doc::nil()
}

fn format_basic_for_statement<'source>(
    statement: &BasicForStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    let initializer = statement
        .initializer()
        .map(|initializer| format_for_initializer(&initializer, doc));
    let condition = statement
        .condition()
        .map(|condition| format_expression(&condition, doc));
    let update = statement
        .update()
        .map(|update| format_for_update(&update, doc));
    let is_empty_header = initializer.is_none() && condition.is_none() && update.is_none();
    let header = if is_empty_header {
        doc_concat!(
            doc,
            [
                format_statement_keyword(statement.keyword(), "for", doc),
                doc.space(),
                format_condition_open_paren(open.as_ref(), doc),
                format_inline_open_paren_spacing(open.as_ref(), doc),
                format_for_header_semicolon(statement.first_semicolon(), doc),
                format_for_header_semicolon(statement.second_semicolon(), doc),
                format_inline_close_paren(close.as_ref(), doc),
            ]
        )
    } else {
        doc_group!(
            doc,
            doc_concat!(
                doc,
                [
                    format_statement_keyword(statement.keyword(), "for", doc),
                    doc.space(),
                    format_condition_open_paren(open.as_ref(), doc),
                    doc_indent!(
                        doc,
                        doc_concat!(
                            doc,
                            [
                                format_for_header_open_spacing(open.as_ref(), doc),
                                format_basic_for_header_clauses(
                                    doc,
                                    initializer,
                                    statement.first_semicolon(),
                                    condition,
                                    statement.second_semicolon(),
                                    update,
                                ),
                            ]
                        )
                    ),
                    format_for_header_close_paren(close.as_ref(), doc),
                ]
            )
        )
    };

    doc_concat!(
        doc,
        [
            header,
            format_statement_header_body_separator(close.as_ref(), doc),
            statement_body_as_block(statement.statement_body().as_ref(), doc),
        ]
    )
}

fn format_basic_for_header_clauses<'source>(
    doc: &mut DocBuilder<'source>,
    initializer: Option<Doc<'source>>,
    first_semicolon: Option<JavaSyntaxToken<'source>>,
    condition: Option<Doc<'source>>,
    second_semicolon: Option<JavaSyntaxToken<'source>>,
    update: Option<Doc<'source>>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            initializer.unwrap_or_else(Doc::nil),
            match first_semicolon {
                Some(semicolon) => {
                    let line = doc.line();
                    format_separator_with_comments(doc, &semicolon, line)
                }
                None => Doc::nil(),
            },
            condition.unwrap_or_else(Doc::nil),
            match second_semicolon {
                Some(semicolon) => {
                    let line = doc.line();
                    format_separator_with_comments(doc, &semicolon, line)
                }
                None => Doc::nil(),
            },
            update.unwrap_or_else(Doc::nil),
        ]
    )
}

fn format_enhanced_for_statement<'source>(
    statement: &EnhancedForStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.keyword(), "for", doc),
            doc.space(),
            doc_group!(
                doc,
                doc_concat!(
                    doc,
                    [
                        format_condition_open_paren(open.as_ref(), doc),
                        doc_indent!(
                            doc,
                            doc_concat!(
                                doc,
                                [
                                    format_for_header_open_spacing(open.as_ref(), doc),
                                    statement.variable().map_or_else(Doc::nil, |variable| {
                                        format_local_variable_declaration(&variable, doc)
                                    },),
                                    doc.space(),
                                    statement.colon().as_ref().map_or_else(Doc::nil, |token| {
                                        format_token_with_comments(doc, token)
                                    },),
                                    doc.space(),
                                    statement.iterable().map_or_else(Doc::nil, |iterable| {
                                        format_expression(&iterable, doc)
                                    },),
                                ],
                            ),
                        ),
                        format_for_header_close_paren(close.as_ref(), doc),
                    ]
                )
            ),
            format_statement_header_body_separator(close.as_ref(), doc),
            statement_body_as_block(statement.statement_body().as_ref(), doc),
        ]
    )
}

fn format_for_header_open_spacing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if open.is_some_and(|open| !open.trailing_comments().is_empty()) {
        format_condition_open_spacing(open, doc)
    } else {
        doc.soft_line()
    }
}

fn format_inline_close_paren<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    doc_concat!(
        doc,
        [
            if close_has_leading_comments {
                doc.line()
            } else {
                Doc::nil()
            },
            close.map_or_else(Doc::nil, |close| {
                doc_concat!(
                    doc,
                    [
                        format_token(
                            doc,
                            close,
                            LeadingTrivia::Preserve,
                            TrailingTrivia::BeforeLineBreak,
                        ),
                        if trailing_comments_force_line(close) {
                            doc.hard_line()
                        } else {
                            Doc::nil()
                        },
                    ]
                )
            },),
        ]
    )
}

fn format_inline_open_paren_spacing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(open) = open else {
        return Doc::nil();
    };

    if open.trailing_comments().is_empty() {
        return Doc::nil();
    }

    doc_concat!(
        doc,
        [
            format_trailing_comments_before_line_break(doc, open),
            if trailing_comments_force_line(open) {
                doc.hard_line()
            } else {
                doc.space()
            },
        ]
    )
}

fn format_for_header_semicolon<'source>(
    semicolon: Option<JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(semicolon) = semicolon else {
        return Doc::nil();
    };

    doc_concat!(
        doc,
        [
            format_token(
                doc,
                &semicolon,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            ),
            if trailing_comments_force_line(&semicolon) {
                doc.hard_line()
            } else {
                Doc::nil()
            },
        ]
    )
}

fn format_for_header_close_paren<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    doc_concat!(
        doc,
        [
            if close_has_leading_comments {
                doc.line()
            } else {
                doc.soft_line()
            },
            close.map_or_else(Doc::nil, |close| {
                doc_concat!(
                    doc,
                    [
                        format_token(
                            doc,
                            close,
                            LeadingTrivia::Preserve,
                            TrailingTrivia::BeforeLineBreak,
                        ),
                        if trailing_comments_force_line(close) {
                            doc.hard_line()
                        } else {
                            Doc::nil()
                        },
                    ]
                )
            },),
        ]
    )
}

fn format_for_initializer<'source>(
    initializer: &ForInitializer<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(declaration) = initializer.local_variable_declaration() {
        return format_local_variable_declaration(&declaration, doc);
    }
    initializer
        .expressions()
        .map_or_else(Doc::nil, |expressions| {
            format_statement_expression_list(&expressions, doc)
        })
}

fn format_for_update<'source>(
    update: &ForUpdate<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    update.expressions().map_or_else(Doc::nil, |expressions| {
        format_statement_expression_list(&expressions, doc)
    })
}

fn format_statement_expression_list<'source>(
    expressions: &StatementExpressionList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut entries = expressions.entries_with_recovered().peekable();
    let mut docs = doc.list();

    while let Some(entry) = entries.next() {
        let has_next = entries.peek().is_some();
        match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => {
                let expression = format_expression(&entry.expression, doc);
                docs.push(expression, doc);
                if let Some(comma) = entry.comma {
                    let space = doc.space();
                    let comma = format_separator_with_comments(doc, &comma, space);
                    docs.push(comma, doc);
                } else if has_next {
                    docs.push(doc.line(), doc);
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                let recovered = format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                );
                docs.push(recovered, doc);
                if has_next {
                    docs.push(doc.line(), doc);
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                let recovered =
                    format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve);
                docs.push(recovered, doc);
                if has_next {
                    docs.push(doc.line(), doc);
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                let recovered =
                    format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve);
                docs.push(recovered, doc);
                if has_next {
                    docs.push(doc.line(), doc);
                }
            }
        }
    }

    docs.finish(doc)
}

pub(super) fn format_synchronized_statement<'source>(
    statement: &SynchronizedStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    let expression = match statement.expression() {
        Some(expression) => format_expression(&expression, doc),
        None => Doc::nil(),
    };
    let expression =
        format_parenthesized_statement_expression(doc, open.as_ref(), expression, close.as_ref());
    let body = match statement.body() {
        Some(body) => format_block(&body, doc),
        None => empty_block(doc),
    };
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.keyword(), "synchronized", doc),
            doc.space(),
            expression,
            format_statement_header_body_separator(close.as_ref(), doc),
            body,
        ]
    )
}
