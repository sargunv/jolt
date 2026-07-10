use super::{
    AssertStatement, Doc, Expression, ExpressionStatement, JavaSyntaxToken, LabeledStatement,
    LeadingTrivia, ReturnStatement, ThrowStatement, TrailingTrivia, YieldStatement,
    comment_forces_line, format_expression, format_statement, format_token,
    format_token_before_relocated_trailing_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};
use crate::helpers::comments::{comment_is_star_block, format_comment, format_token_sequence};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::JavaComment;

pub(super) fn format_labeled_statement<'source>(
    statement: &LabeledStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let label = statement
        .label()
        .map_or_else(Doc::nil, |label| format_token_with_comments(doc, &label));

    doc_concat!(
        doc,
        [
            label,
            statement
                .colon()
                .as_ref()
                .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
            doc.hard_line(),
            statement
                .body()
                .map_or_else(Doc::nil, |body| format_statement(&body, doc)),
        ]
    )
}

pub(super) fn format_expression_statement<'source>(
    statement: &ExpressionStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(expression) = statement.expression() else {
        return format_token_sequence(doc, statement.token_iter(), LeadingTrivia::Preserve);
    };

    doc_concat!(
        doc,
        [
            format_expression(&expression, doc),
            format_statement_semicolon(statement.semicolon(), doc),
        ]
    )
}
pub(super) fn format_assert_statement<'source>(
    statement: &AssertStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.keyword(), "assert", doc),
            doc.space(),
            statement
                .condition()
                .map_or_else(Doc::nil, |condition| format_expression(&condition, doc),),
            statement.detail().map_or_else(Doc::nil, |detail| {
                doc_concat!(
                    doc,
                    [
                        doc.space(),
                        statement
                            .colon()
                            .as_ref()
                            .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
                        doc.space(),
                        format_expression(&detail, doc),
                    ]
                )
            },),
            format_statement_semicolon(statement.semicolon(), doc),
        ]
    )
}

pub(super) fn format_return_statement<'source>(
    statement: &ReturnStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "return",
        statement.expression(),
        statement.semicolon(),
        doc,
    )
}

pub(super) fn format_throw_statement<'source>(
    statement: &ThrowStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "throw",
        statement.expression(),
        statement.semicolon(),
        doc,
    )
}

pub(super) fn format_yield_statement<'source>(
    statement: &YieldStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "yield",
        statement.expression(),
        statement.semicolon(),
        doc,
    )
}

fn format_keyword_expression_statement<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
    expression: Option<Expression<'source>>,
    semicolon: Option<JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_statement_keyword_head(keyword, fallback, doc),
            expression.map_or_else(Doc::nil, |expression| {
                let expression = format_expression(&expression, doc);
                let expression = doc_concat!(
                    doc,
                    [
                        format_keyword_expression_separator(keyword, doc),
                        expression,
                    ]
                );
                if keyword_expression_separator_forces_line(keyword) {
                    doc_indent!(doc, expression)
                } else {
                    expression
                }
            },),
            format_statement_semicolon(semicolon, doc),
        ]
    )
}

fn keyword_expression_separator_forces_line(keyword: Option<&JavaSyntaxToken<'_>>) -> bool {
    keyword.is_some_and(trailing_comments_force_line)
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
    keyword: Option<JavaSyntaxToken<'source>>,
    fallback: &'static str,
    label: Option<JavaSyntaxToken<'source>>,
    semicolon: Option<JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_statement_keyword(keyword, fallback, doc),
            label.map_or_else(Doc::nil, |label| doc_concat!(
                doc,
                [doc.space(), format_token_with_comments(doc, &label)]
            ),),
            format_statement_semicolon(semicolon, doc),
        ]
    )
}

pub(crate) fn format_statement_semicolon<'source>(
    semicolon: Option<JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(semicolon) = semicolon else {
        return Doc::nil();
    };

    doc_concat!(
        doc,
        [
            format_semicolon_leading_comments(&semicolon, doc),
            format_token(
                doc,
                &semicolon,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            format_terminator_trailing_comments(&semicolon, doc),
        ]
    )
}

fn format_semicolon_leading_comments<'source>(
    semicolon: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in semicolon.leading_comments() {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
            if comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
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
            if terminator_comment_starts_next_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            } else {
                let space = docs.space();
                docs.push(space);
            }
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
        }
    })
}

fn terminator_comment_starts_next_line(comment: &JavaComment<'_>) -> bool {
    comment_is_star_block(comment)
}

pub(super) fn format_statement_keyword<'source>(
    keyword: Option<JavaSyntaxToken<'source>>,
    _fallback: &'static str,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    keyword.map_or_else(Doc::nil, |keyword| {
        format_token_with_comments(doc, &keyword)
    })
}

pub(super) fn format_statement_keyword_head<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    _fallback: &'static str,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    keyword.map_or_else(Doc::nil, |keyword| {
        format_token_before_relocated_trailing_comments(doc, keyword, LeadingTrivia::Preserve)
    })
}
