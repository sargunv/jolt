use super::{
    AssertStatement, Doc, ExpressionStatement, JavaSyntaxToken, LabeledStatement, LeadingTrivia,
    ReturnStatement, ThrowStatement, TrailingTrivia, YieldStatement, comment_forces_line,
    format_expression, format_statement, format_token,
    format_token_before_relocated_trailing_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};
use crate::helpers::comments::{comment_is_star_block, format_comment, format_trailing_comment};
use crate::helpers::recovery::{
    JavaFormatField, format_optional_field, format_required_field, resolve_optional_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{Expression, JavaSyntaxField, JavaSyntaxInvariantError};

type TokenField<'source> =
    Result<JavaSyntaxField<'source, JavaSyntaxToken<'source>>, JavaSyntaxInvariantError>;

pub(super) fn format_labeled_statement<'source>(
    statement: &LabeledStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_required_field(statement.label(), doc, |token, doc| {
                format_token_with_comments(doc, &token)
            }),
            format_required_field(statement.colon(), doc, |token, doc| {
                format_token_with_comments(doc, &token)
            }),
            doc.hard_line(),
            format_required_field(statement.body(), doc, |body, doc| format_statement(
                &body, doc
            )),
        ]
    )
}

pub(super) fn format_expression_statement<'source>(
    statement: &ExpressionStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_required_field(statement.expression(), doc, |expression, doc| {
                format_expression(&expression, doc)
            }),
            format_statement_semicolon(statement.semicolon(), doc),
        ]
    )
}

pub(super) fn format_assert_statement<'source>(
    statement: &AssertStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let message = format_optional_field(statement.message(), doc, |message, doc| {
        doc_concat!(
            doc,
            [
                doc.space(),
                format_optional_field(statement.colon(), doc, |token, doc| {
                    format_token_with_comments(doc, &token)
                }),
                doc.space(),
                format_expression(&message, doc),
            ]
        )
    });
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.assert_keyword(), doc),
            doc.space(),
            format_required_field(statement.condition(), doc, |condition, doc| {
                format_expression(&condition, doc)
            }),
            message,
            format_statement_semicolon(statement.semicolon(), doc),
        ]
    )
}

pub(super) fn format_return_statement<'source>(
    statement: &ReturnStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_keyword_expression_statement(
        statement.return_keyword(),
        statement.expression(),
        statement.semicolon(),
        doc,
    )
}

pub(super) fn format_throw_statement<'source>(
    statement: &ThrowStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_keyword_expression_statement(
        statement.throw_keyword(),
        statement.expression(),
        statement.semicolon(),
        doc,
    )
}

pub(super) fn format_yield_statement<'source>(
    statement: &YieldStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_keyword_expression_statement(
        statement.yield_keyword(),
        statement.expression(),
        statement.semicolon(),
        doc,
    )
}

fn format_keyword_expression_statement<'source>(
    keyword: TokenField<'source>,
    expression: Result<JavaSyntaxField<'source, Expression<'source>>, JavaSyntaxInvariantError>,
    semicolon: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let keyword_token = match keyword {
        Ok(JavaSyntaxField::Present(token)) => Some(token),
        _ => None,
    };
    let head = format_statement_keyword_head(keyword, doc);
    let expression = match resolve_optional_field(expression, doc) {
        JavaFormatField::Present(Some(expression)) => {
            let separator = format_keyword_expression_separator(keyword_token.as_ref(), doc);
            let expression = doc_concat!(doc, [separator, format_expression(&expression, doc)]);
            if keyword_token
                .as_ref()
                .is_some_and(trailing_comments_force_line)
            {
                doc_indent!(doc, expression)
            } else {
                expression
            }
        }
        JavaFormatField::Present(None) => keyword_token.as_ref().map_or_else(Doc::nil, |token| {
            if token.trailing_comments().is_empty() {
                Doc::nil()
            } else {
                doc_concat!(
                    doc,
                    [
                        format_trailing_comments_before_line_break(doc, token),
                        if trailing_comments_force_line(token) {
                            doc.hard_line()
                        } else {
                            Doc::nil()
                        },
                    ]
                )
            }
        }),
        JavaFormatField::Malformed(recovery) => recovery,
    };
    doc_concat!(
        doc,
        [head, expression, format_statement_semicolon(semicolon, doc)]
    )
}

fn format_required_keyword_expression_statement<'source>(
    keyword: TokenField<'source>,
    expression: Result<JavaSyntaxField<'source, Expression<'source>>, JavaSyntaxInvariantError>,
    semicolon: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let keyword_token = match keyword {
        Ok(JavaSyntaxField::Present(token)) => Some(token),
        _ => None,
    };
    let head = format_statement_keyword_head(keyword, doc);
    let expression = format_required_field(expression, doc, |expression, doc| {
        doc_concat!(
            doc,
            [
                format_keyword_expression_separator(keyword_token.as_ref(), doc),
                format_expression(&expression, doc)
            ]
        )
    });
    doc_concat!(
        doc,
        [head, expression, format_statement_semicolon(semicolon, doc)]
    )
}

fn format_keyword_expression_separator<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(keyword) = keyword else {
        return doc.space();
    };
    if keyword.trailing_comments().is_empty() {
        return doc.space();
    }
    doc_concat!(
        doc,
        [
            format_trailing_comments_before_line_break(doc, keyword),
            if trailing_comments_force_line(keyword) {
                doc.hard_line()
            } else {
                doc.space()
            },
        ]
    )
}

pub(super) fn format_jump_statement<'source>(
    keyword: TokenField<'source>,
    label: TokenField<'source>,
    semicolon: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_statement_keyword(keyword, doc),
            format_optional_field(label, doc, |label, doc| doc_concat!(
                doc,
                [doc.space(), format_token_with_comments(doc, &label)]
            )),
            format_statement_semicolon(semicolon, doc),
        ]
    )
}

pub(crate) fn format_statement_semicolon<'source>(
    semicolon: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(semicolon, doc, |semicolon, doc| {
        doc_concat!(
            doc,
            [
                format_semicolon_leading_comments(&semicolon, doc),
                format_token(
                    doc,
                    &semicolon,
                    LeadingTrivia::SuppressAlreadyHandled,
                    TrailingTrivia::RelocatedToEnclosingContext
                ),
                format_terminator_trailing_comments(&semicolon, doc),
            ]
        )
    })
}

fn format_semicolon_leading_comments<'source>(
    semicolon: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in semicolon.leading_comments() {
            let space = docs.space();
            docs.push(space);
            let formatted = format_comment(docs, &comment);
            docs.push(formatted);
            if comment_forces_line(&comment) {
                let line = docs.hard_line();
                docs.push(line);
            }
        }
    })
}

fn format_terminator_trailing_comments<'source>(
    token: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in token.trailing_comments() {
            let multiline_star =
                comment_is_star_block(&comment) && comment.text().contains(['\n', '\r']);
            let separator = if multiline_star {
                docs.hard_line()
            } else {
                docs.space()
            };
            docs.push(separator);
            let formatted = format_trailing_comment(docs, &comment);
            docs.push(formatted);
        }
    })
}

pub(super) fn format_statement_keyword<'source>(
    keyword: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(keyword, doc, |keyword, doc| {
        format_token_with_comments(doc, &keyword)
    })
}

pub(super) fn format_statement_keyword_head<'source>(
    keyword: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(keyword, doc, |keyword, doc| {
        format_token_before_relocated_trailing_comments(doc, &keyword, LeadingTrivia::Preserve)
    })
}
