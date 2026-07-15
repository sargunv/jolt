use super::simple::format_statement_keyword;
use super::{
    BasicForStatement, DoStatement, EnhancedForStatement, ForStatement, IfStatement,
    JavaSyntaxToken, LeadingTrivia, Statement, SynchronizedStatement, TrailingTrivia,
    WhileStatement, format_block, format_expression, format_local_variable_declaration,
    format_separator_with_comments, format_statement_semicolon, format_token,
    format_token_with_comments, format_trailing_comments_before_line_break,
    statement_body_as_block, statement_body_trailing_comments_force_line,
    trailing_comments_force_line,
};
use crate::helpers::recovery::{
    JavaFormatDelimiter, JavaFormatField, format_optional_field, format_or_verbatim,
    format_required_field, resolve_optional_field, resolve_required_delimiter,
    resolve_required_field,
};
use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    ForInitializer, ForUpdate, JavaSyntaxField, LocalVariableDeclaration, StatementExpressionList,
};

pub(super) fn format_if_statement<'source>(
    statement: &IfStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        let else_branch = resolve_optional_field(statement.else_branch(), doc);
        let (then_doc, then_forces_line) =
            match resolve_required_field(statement.then_branch(), doc) {
                JavaFormatField::Present(branch) => {
                    let forces_line = matches!(else_branch, JavaFormatField::Present(Some(_)))
                        && statement_body_trailing_comments_force_line(&branch);
                    (statement_as_block(&branch, doc), forces_line)
                }
                JavaFormatField::Malformed(recovery) => (recovery, false),
            };
        let close = resolve_required_delimiter(statement.close_paren(), doc);
        let separator = format_statement_header_body_separator(close.source(), doc);
        let condition = format_required_field(statement.condition(), doc, |condition, doc| {
            format_expression(&condition, doc)
        });
        let open = resolve_required_delimiter(statement.open_paren(), doc);
        let condition = format_parenthesized_statement_expression(doc, open, condition, close);
        let else_doc = match else_branch {
            JavaFormatField::Present(Some(Statement::IfStatement(else_if))) => doc_concat!(
                doc,
                [
                    if then_forces_line {
                        Doc::nil()
                    } else {
                        doc.space()
                    },
                    format_optional_field(statement.else_keyword(), doc, |token, doc| {
                        format_token_with_comments(doc, &token)
                    }),
                    doc.space(),
                    format_if_statement(&else_if, doc),
                ]
            ),
            JavaFormatField::Present(Some(branch)) => doc_concat!(
                doc,
                [
                    if then_forces_line {
                        Doc::nil()
                    } else {
                        doc.space()
                    },
                    format_optional_field(statement.else_keyword(), doc, |token, doc| {
                        format_token_with_comments(doc, &token)
                    }),
                    doc.space(),
                    statement_as_block(&branch, doc),
                ]
            ),
            JavaFormatField::Present(None) => {
                format_optional_field(statement.else_keyword(), doc, |token, doc| {
                    format_token_with_comments(doc, &token)
                })
            }
            JavaFormatField::Malformed(recovery) => recovery,
        };
        doc_concat!(
            doc,
            [
                format_statement_keyword(statement.if_keyword(), doc),
                doc.space(),
                condition,
                separator,
                then_doc,
                else_doc,
            ]
        )
    })
}

fn statement_as_block<'source>(
    statement: &Statement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    statement_body_as_block(Ok(JavaSyntaxField::Present(*statement)), doc)
}

pub(super) fn format_parenthesized_statement_expression<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    expression: Doc<'source>,
    close: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    let open_spacing = format_condition_open_spacing(open.source(), doc);
    let open = format_condition_open_paren(open, doc);
    let close = format_condition_close_paren(close, doc);
    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                open,
                doc_indent!(doc, doc_concat!(doc, [open_spacing, expression])),
                close
            ]
        )
    )
}

pub(super) fn format_condition_open_paren<'source>(
    open: JavaFormatDelimiter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match open {
        JavaFormatDelimiter::Source(open) => format_token(
            doc,
            &open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        JavaFormatDelimiter::Recovery(recovery) => recovery,
    }
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
    close: JavaFormatDelimiter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let has_leading = close
        .source()
        .is_some_and(|token| !token.leading_comments().is_empty());
    let close = match close {
        JavaFormatDelimiter::Source(close) => doc_concat!(
            doc,
            [
                format_token(
                    doc,
                    &close,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak
                ),
                if trailing_comments_force_line(&close) {
                    doc.hard_line()
                } else {
                    Doc::nil()
                },
            ]
        ),
        JavaFormatDelimiter::Recovery(recovery) => recovery,
    };
    doc_concat!(
        doc,
        [
            if has_leading {
                doc.line()
            } else {
                doc.soft_line()
            },
            close
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
    format_or_verbatim(statement, doc, |doc| {
        let close = resolve_required_delimiter(statement.close_paren(), doc);
        let separator = format_statement_header_body_separator(close.source(), doc);
        let condition = format_required_field(statement.condition(), doc, |value, doc| {
            format_expression(&value, doc)
        });
        let open = resolve_required_delimiter(statement.open_paren(), doc);
        let condition = format_parenthesized_statement_expression(doc, open, condition, close);
        doc_concat!(
            doc,
            [
                format_statement_keyword(statement.while_keyword(), doc),
                doc.space(),
                condition,
                separator,
                statement_body_as_block(statement.body(), doc),
            ]
        )
    })
}

pub(super) fn format_do_statement<'source>(
    statement: &DoStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        let condition = format_required_field(statement.condition(), doc, |value, doc| {
            format_expression(&value, doc)
        });
        let open = resolve_required_delimiter(statement.open_paren(), doc);
        let close = resolve_required_delimiter(statement.close_paren(), doc);
        let condition = format_parenthesized_statement_expression(doc, open, condition, close);
        doc_concat!(
            doc,
            [
                format_statement_keyword(statement.do_keyword(), doc),
                doc.space(),
                statement_body_as_block(statement.body(), doc),
                doc.space(),
                format_statement_keyword(statement.while_keyword(), doc),
                doc.space(),
                condition,
                format_statement_semicolon(statement.semicolon(), doc),
            ]
        )
    })
}

pub(super) fn format_for_statement<'source>(
    statement: &ForStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        format_required_field(statement.form(), doc, |form, doc| {
            if let Some(basic) = form.cast_node::<BasicForStatement<'source>>() {
                format_basic_for_statement(&basic, doc)
            } else if let Some(enhanced) = form.cast_node::<EnhancedForStatement<'source>>() {
                format_enhanced_for_statement(&enhanced, doc)
            } else {
                doc.block_on_invariant("for statement form contradicted its declared role");
                Doc::nil()
            }
        })
    })
}

fn format_basic_for_statement<'source>(
    statement: &BasicForStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        let initializer = match resolve_optional_field(statement.initializer(), doc) {
            JavaFormatField::Present(value) => {
                value.map(|value| format_for_initializer(&value, doc))
            }
            JavaFormatField::Malformed(recovery) => Some(recovery),
        };
        let condition = match resolve_optional_field(statement.condition(), doc) {
            JavaFormatField::Present(value) => value.map(|value| format_expression(&value, doc)),
            JavaFormatField::Malformed(recovery) => Some(recovery),
        };
        let update = match resolve_optional_field(statement.update(), doc) {
            JavaFormatField::Present(value) => value.map(|value| format_for_update(&value, doc)),
            JavaFormatField::Malformed(recovery) => Some(recovery),
        };
        let is_empty_header = initializer.is_none() && condition.is_none() && update.is_none();
        let open = resolve_required_delimiter(statement.open_paren(), doc);
        let close = resolve_required_delimiter(statement.close_paren(), doc);
        let separator = format_statement_header_body_separator(close.source(), doc);
        let header = if is_empty_header {
            let open_spacing = format_inline_open_paren_spacing(open.source(), doc);
            doc_concat!(
                doc,
                [
                    format_statement_keyword(statement.for_keyword(), doc),
                    doc.space(),
                    format_condition_open_paren(open, doc),
                    open_spacing,
                    format_for_header_semicolon(statement.first_semicolon(), doc),
                    format_for_header_semicolon(statement.second_semicolon(), doc),
                    format_inline_close_paren(close, doc),
                ]
            )
        } else {
            let open_spacing = format_condition_open_spacing(open.source(), doc);
            let clauses = doc_concat!(
                doc,
                [
                    initializer.unwrap_or_else(Doc::nil),
                    format_required_field(statement.first_semicolon(), doc, |token, doc| {
                        let line = doc.line();
                        format_separator_with_comments(doc, &token, line)
                    }),
                    condition.unwrap_or_else(Doc::nil),
                    format_required_field(statement.second_semicolon(), doc, |token, doc| {
                        let line = doc.line();
                        format_separator_with_comments(doc, &token, line)
                    }),
                    update.unwrap_or_else(Doc::nil),
                ]
            );
            let contents = doc_indent!(doc, doc_concat!(doc, [open_spacing, clauses]));
            let open = format_condition_open_paren(open, doc);
            let close = format_condition_close_paren(close, doc);
            doc_group!(
                doc,
                doc_concat!(
                    doc,
                    [
                        format_statement_keyword(statement.for_keyword(), doc),
                        doc.space(),
                        open,
                        contents,
                        close,
                    ]
                )
            )
        };
        doc_concat!(
            doc,
            [
                header,
                separator,
                statement_body_as_block(statement.body(), doc)
            ]
        )
    })
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

fn format_inline_close_paren<'source>(
    close: JavaFormatDelimiter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match close {
        JavaFormatDelimiter::Source(close) => {
            let leading = if close.leading_comments().is_empty() {
                Doc::nil()
            } else {
                doc.line()
            };
            let trailing = if trailing_comments_force_line(&close) {
                doc.hard_line()
            } else {
                Doc::nil()
            };
            let close = format_token(
                doc,
                &close,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            );
            doc_concat!(doc, [leading, close, trailing])
        }
        JavaFormatDelimiter::Recovery(recovery) => recovery,
    }
}

fn format_for_header_semicolon<'source>(
    field: Result<
        JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(field, doc, |semicolon, doc| {
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
    })
}

fn format_enhanced_for_statement<'source>(
    statement: &EnhancedForStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        let contents = doc_concat!(
            doc,
            [
                format_required_field(statement.variable(), doc, |value, doc| {
                    format_local_variable_declaration(&value, doc)
                }),
                doc.space(),
                format_required_field(statement.colon(), doc, |token, doc| {
                    format_token_with_comments(doc, &token)
                }),
                doc.space(),
                format_required_field(statement.iterable(), doc, |value, doc| format_expression(
                    &value, doc
                )),
            ]
        );
        let close = resolve_required_delimiter(statement.close_paren(), doc);
        let separator = format_statement_header_body_separator(close.source(), doc);
        let open = resolve_required_delimiter(statement.open_paren(), doc);
        let header = format_parenthesized_statement_expression(doc, open, contents, close);
        doc_concat!(
            doc,
            [
                format_statement_keyword(statement.for_keyword(), doc),
                doc.space(),
                header,
                separator,
                statement_body_as_block(statement.body(), doc),
            ]
        )
    })
}

fn format_for_initializer<'source>(
    value: &ForInitializer<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(value, doc, |doc| {
        format_required_field(value.value(), doc, |value, doc| {
            if let Some(declaration) = value.cast_node::<LocalVariableDeclaration<'source>>() {
                format_local_variable_declaration(&declaration, doc)
            } else if let Some(expressions) = value.cast_node::<StatementExpressionList<'source>>()
            {
                format_statement_expression_list(&expressions, doc)
            } else {
                doc.block_on_invariant("for initializer contradicted its declared role");
                Doc::nil()
            }
        })
    })
}

fn format_for_update<'source>(
    value: &ForUpdate<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(value, doc, |doc| {
        format_required_field(value.expressions(), doc, |list, doc| {
            format_statement_expression_list(&list, doc)
        })
    })
}

fn format_statement_expression_list<'source>(
    list: &StatementExpressionList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(list, doc, |doc| {
        doc.concat_list(|docs| {
            for part in list.parts() {
                match crate::helpers::recovery::resolve_list_part(part, docs) {
                    crate::helpers::recovery::JavaFormatListPart::Item(expression) => {
                        let expression = format_expression(&expression, docs);
                        docs.push(expression);
                    }
                    crate::helpers::recovery::JavaFormatListPart::Separator(comma) => {
                        let space = docs.space();
                        let comma = format_separator_with_comments(docs, &comma, space);
                        docs.push(comma);
                    }
                    crate::helpers::recovery::JavaFormatListPart::Malformed(recovery) => {
                        docs.push(recovery);
                    }
                }
            }
        })
    })
}

pub(super) fn format_synchronized_statement<'source>(
    statement: &SynchronizedStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        let expression = format_required_field(statement.expression(), doc, |value, doc| {
            format_expression(&value, doc)
        });
        let close = resolve_required_delimiter(statement.close_paren(), doc);
        let separator = format_statement_header_body_separator(close.source(), doc);
        let open = resolve_required_delimiter(statement.open_paren(), doc);
        let expression = format_parenthesized_statement_expression(doc, open, expression, close);
        doc_concat!(
            doc,
            [
                format_statement_keyword(statement.synchronized_keyword(), doc),
                doc.space(),
                expression,
                separator,
                format_required_field(statement.body(), doc, |body, doc| format_block(&body, doc)),
            ]
        )
    })
}
