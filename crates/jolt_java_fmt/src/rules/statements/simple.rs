use super::{
    AssertStatement, Doc, ExpressionStatement, JavaSyntaxToken, LabeledStatement, LeadingTrivia,
    ReturnStatement, ThrowStatement, TrailingTrivia, YieldStatement, comment_forces_line,
    format_expression, format_statement, format_token,
    format_token_before_relocated_trailing_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};
use crate::helpers::comments::{
    comment_is_star_block, format_comment, format_construct_leading_comments,
    format_token_after_relocated_leading_comments, format_trailing_comment,
};
use crate::helpers::recovery::{
    JavaFormatField, format_optional_field, format_required_field, resolve_optional_field,
    resolve_required_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{Expression, JavaSyntaxField};

type TokenField<'source> = JavaSyntaxField<'source, JavaSyntaxToken<'source>>;

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
    let keyword = statement.assert_keyword();
    let keyword_token = present_keyword(keyword);
    let head = format_statement_keyword_head(keyword, doc);
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
    let condition = match resolve_required_field(statement.condition(), doc) {
        JavaFormatField::Present(condition) => {
            let separator = format_keyword_operand_separator(keyword_token.as_ref(), doc);
            let condition = format_expression(&condition, doc);
            doc_concat!(doc, [separator, condition])
        }
        JavaFormatField::Malformed(recovery) => {
            let comments = format_orphaned_keyword_comments(keyword_token.as_ref(), doc);
            doc_concat!(doc, [comments, recovery])
        }
    };
    let operand = indent_keyword_continuation(
        keyword_token.as_ref(),
        doc_concat!(doc, [condition, message]),
        doc,
    );
    doc_concat!(
        doc,
        [
            head,
            operand,
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
    expression: JavaSyntaxField<'source, Expression<'source>>,
    semicolon: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let operand = resolve_optional_field(expression, doc);
    format_keyword_operand_statement(keyword, operand, semicolon, doc, |expression, doc| {
        format_expression(&expression, doc)
    })
}

fn format_required_keyword_expression_statement<'source>(
    keyword: TokenField<'source>,
    expression: JavaSyntaxField<'source, Expression<'source>>,
    semicolon: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let operand = into_optional_operand(resolve_required_field(expression, doc));
    format_keyword_operand_statement(keyword, operand, semicolon, doc, |expression, doc| {
        format_expression(&expression, doc)
    })
}

pub(super) fn format_jump_statement<'source>(
    keyword: TokenField<'source>,
    label: TokenField<'source>,
    semicolon: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let operand = resolve_optional_field(label, doc);
    format_keyword_operand_statement(keyword, operand, semicolon, doc, |label, doc| {
        format_token_with_comments(doc, &label)
    })
}

/// Formats a statement built from a keyword, the operand it applies to, and a
/// terminator.
///
/// The keyword's trailing comments are emitted here rather than on the keyword
/// token itself, because they decide where the operand goes: a comment that
/// ends the keyword's line pushes the operand onto the next one.
fn format_keyword_operand_statement<'source, T>(
    keyword: TokenField<'source>,
    operand: JavaFormatField<'source, Option<T>>,
    semicolon: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
    format_operand: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    let keyword_token = present_keyword(keyword);
    let head = format_statement_keyword_head(keyword, doc);
    let operand = match operand {
        JavaFormatField::Present(Some(operand)) => {
            let separator = format_keyword_operand_separator(keyword_token.as_ref(), doc);
            let operand = format_operand(operand, doc);
            let operand = doc_concat!(doc, [separator, operand]);
            indent_keyword_continuation(keyword_token.as_ref(), operand, doc)
        }
        JavaFormatField::Present(None) => {
            format_orphaned_keyword_comments(keyword_token.as_ref(), doc)
        }
        JavaFormatField::Malformed(recovery) => {
            let comments = format_orphaned_keyword_comments(keyword_token.as_ref(), doc);
            doc_concat!(doc, [comments, recovery])
        }
    };
    doc_concat!(
        doc,
        [head, operand, format_statement_semicolon(semicolon, doc)]
    )
}

fn into_optional_operand<T>(field: JavaFormatField<'_, T>) -> JavaFormatField<'_, Option<T>> {
    match field {
        JavaFormatField::Present(operand) => JavaFormatField::Present(Some(operand)),
        JavaFormatField::Malformed(recovery) => JavaFormatField::Malformed(recovery),
    }
}

fn present_keyword(keyword: TokenField<'_>) -> Option<JavaSyntaxToken<'_>> {
    match keyword {
        JavaSyntaxField::Present(token) => Some(token),
        JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => None,
    }
}

/// Indents an operand that a comment moved off its keyword's line.
///
/// The comment's own hard line sits inside the indent, so the operand lands one
/// level in and reads as the keyword's continuation rather than as a statement
/// of its own. Without such a comment there is no line to indent.
fn indent_keyword_continuation<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    operand: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if keyword.is_some_and(trailing_comments_force_line) {
        doc_indent!(doc, operand)
    } else {
        operand
    }
}

fn format_keyword_operand_separator<'source>(
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

/// Emits a keyword's trailing comments when no operand follows to carry them.
fn format_orphaned_keyword_comments<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(keyword) = keyword else {
        return Doc::nil();
    };
    if keyword.trailing_comments().is_empty() {
        return Doc::nil();
    }
    doc_concat!(
        doc,
        [
            format_trailing_comments_before_line_break(doc, keyword),
            if trailing_comments_force_line(keyword) {
                doc.hard_line()
            } else {
                Doc::nil()
            },
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

/// Splits a statement keyword's leading comments from the keyword itself.
///
/// A leading comment ends its own line. Left inside a statement's header group,
/// that hard line forces the whole header to break with it, so a comment above
/// the statement would silently change its layout. The caller places the
/// returned comments outside the group.
pub(super) fn format_statement_keyword_hoisting_leading<'source>(
    keyword: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, Doc<'source>) {
    let token = match keyword {
        jolt_java_syntax::JavaSyntaxField::Present(token) => Some(token),
        jolt_java_syntax::JavaSyntaxField::Missing(_)
        | jolt_java_syntax::JavaSyntaxField::Malformed(_) => None,
    };
    let leading = format_construct_leading_comments(doc, token.as_ref());
    let keyword = format_required_field(keyword, doc, |keyword, doc| {
        format_token_after_relocated_leading_comments(doc, &keyword, TrailingTrivia::Preserve)
    });
    (leading, keyword)
}

pub(super) fn format_statement_keyword_head<'source>(
    keyword: TokenField<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(keyword, doc, |keyword, doc| {
        format_token_before_relocated_trailing_comments(doc, &keyword, LeadingTrivia::Preserve)
    })
}
